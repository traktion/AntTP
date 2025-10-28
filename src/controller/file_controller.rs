use actix_http::header::HeaderMap;
use actix_web::{web, HttpRequest, HttpResponse, HttpResponseBuilder};
use actix_web::dev::ConnectionInfo;
use actix_web::web::Data;
use log::debug;
use mime::{Mime, APPLICATION_JSON, TEXT_HTML};
use crate::config::anttp_config::AntTpConfig;
use crate::{UploadState, UploaderState};
use crate::service::public_archive_service::PublicArchiveService;
use crate::client::CachingClient;
use crate::error::GetError;
use crate::error::chunk_error::ChunkError;
use crate::service::archive_helper::{ArchiveAction, ArchiveHelper, ArchiveInfo};
use crate::service::file_service::{FileService, RangeProps};
use crate::service::header_builder::HeaderBuilder;
use crate::service::resolver_service::{ResolvedAddress, ResolverService};

pub async fn get_public_data(
    request: HttpRequest,
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    conn: ConnectionInfo,
    uploader_state_data: Data<UploaderState>,
    upload_state_data: Data<UploadState>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> Result<HttpResponse, ChunkError> {
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let caching_client = caching_client_data.get_ref().clone();
    let resolver_service = ResolverService::new(ant_tp_config.clone(), caching_client.clone());

    match resolver_service.resolve(&conn.host(), &path.into_inner(), &request.headers()).await {
        Some(resolved_address) => {
            let header_builder = HeaderBuilder::new(ant_tp_config.cached_mutable_ttl);
            if !resolved_address.is_modified {
                Ok(build_not_modified_response(&resolved_address, &header_builder))
            } else if resolved_address.archive.is_some() {
                debug!("Retrieving file from archive [{:x}]", resolved_address.xor_name);
                let file_service = FileService::new(caching_client.clone(), ant_tp_config.clone());
                let public_archive_service = PublicArchiveService::new(
                    file_service, uploader_state_data, upload_state_data, caching_client);
                let archive_info = public_archive_service.get_archive_info(&resolved_address, &request).await;

                match archive_info.action {
                    ArchiveAction::Data => get_data_archive(&request, &resolved_address, &header_builder, public_archive_service, archive_info).await,
                    ArchiveAction::Redirect => Ok(build_moved_permanently_response(&request.path(), &header_builder)),
                    ArchiveAction::Listing  => Ok(build_list_files_response(&request, &resolved_address, &header_builder)),
                    ArchiveAction::NotFound => Err(GetError::RecordNotFound(format!("File not found: {}", request.full_url())).into()),
                }
            } else {
                debug!("Retrieving file from XOR [{:x}]", resolved_address.xor_name);
                let file_service = FileService::new(caching_client.clone(), ant_tp_config.clone());
                get_data_xor(&request, &resolved_address, &header_builder, file_service).await
            }
        },
        None => Err(GetError::RecordNotFound(format!("File not found: {}", request.full_url())).into())
    }
}

fn build_not_modified_response(resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder) -> HttpResponse {
    HttpResponse::NotModified()
        .insert_header(header_builder.build_cache_control_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_expires_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .finish()
}

fn build_moved_permanently_response(request_path: &str, header_builder: &HeaderBuilder) -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header(header_builder.build_location_header(format!("{}/", request_path)))
        .insert_header(header_builder.build_server_header())
        .finish()
}

fn build_list_files_response(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder) -> HttpResponse {
    let archive_helper = ArchiveHelper::new(resolved_address.archive.clone().unwrap());
    let mime = get_accept_header_value(request.headers());
    if mime == APPLICATION_JSON {
        HttpResponse::Ok()
            .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
            .insert_header(header_builder.build_cors_header())
            .insert_header(header_builder.build_server_header())
            .insert_header(header_builder.build_content_type_header_from_mime(&mime))
            .body(archive_helper.list_files(resolved_address.file_path.clone(), request.headers()))
    } else {
        HttpResponse::Ok()
            // can only use etag for one content-type currently. JSON can have priority as could cause app issues.
            .insert_header(header_builder.build_cors_header())
            .insert_header(header_builder.build_server_header())
            .insert_header(header_builder.build_content_type_header_from_mime(&mime))
            .body(archive_helper.list_files(resolved_address.file_path.clone(), request.headers()))
    }
}

fn update_partial_content_response(builder: &mut HttpResponseBuilder, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, range_props: &RangeProps) {
    builder
        .insert_header(header_builder.build_content_range_header(range_props.range_from().unwrap(), range_props.range_to().unwrap(), range_props.content_length()))
        .insert_header(header_builder.build_accept_ranges_header())
        .insert_header(header_builder.build_cache_control_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_expires_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header(range_props.extension()));
}

fn update_full_content_response(builder: &mut HttpResponseBuilder, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, range_props: &RangeProps) {
    builder
        .insert_header(header_builder.build_content_length_header(range_props.content_length()))
        .insert_header(header_builder.build_cache_control_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_expires_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header(range_props.extension()));
}

async fn get_data_archive(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, public_archive_service: PublicArchiveService, archive_info: ArchiveInfo) -> Result<HttpResponse, ChunkError> {
    let (chunk_receiver, range_props) = public_archive_service.get_data(&request, archive_info).await?;
    if range_props.is_range() {
        let mut builder = HttpResponse::PartialContent();
        update_partial_content_response(&mut builder, &resolved_address, &header_builder, &range_props);
        Ok(builder.streaming(chunk_receiver))
    } else {
        let mut builder = HttpResponse::Ok();
        update_full_content_response(&mut builder, &resolved_address, &header_builder, &range_props);
        Ok(builder.streaming(chunk_receiver))
    }
}

fn get_accept_header_value(header_map: &HeaderMap) -> Mime {
    if header_map.contains_key("Accept")
        && header_map.get("Accept").unwrap().to_str().unwrap_or("").to_string().contains( "json") {
        APPLICATION_JSON
    } else {
        TEXT_HTML
    }
}

async fn get_data_xor(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, file_service: FileService) -> Result<HttpResponse, ChunkError> {
    let (chunk_receiver, range_props) = file_service.get_data(&request, &resolved_address).await?;
    if range_props.is_range() {
        let mut builder = HttpResponse::PartialContent();
        update_partial_content_response(&mut builder, &resolved_address, &header_builder, &range_props);
        Ok(builder.streaming(chunk_receiver))
    } else {
        let mut builder = HttpResponse::Ok();
        update_full_content_response(&mut builder, &resolved_address, &header_builder, &range_props);
        Ok(builder.streaming(chunk_receiver))
    }
}


