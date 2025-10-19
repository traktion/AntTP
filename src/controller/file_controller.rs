use actix_web::{web, Error, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};
use actix_web::dev::ConnectionInfo;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::web::Data;
use log::debug;
use crate::config::anttp_config::AntTpConfig;
use crate::{UploaderState, UploadState};
use crate::service::public_archive_service::PublicArchiveService;
use crate::client::CachingClient;
use crate::client::error::ChunkError;
use crate::controller::handle_get_error;
use crate::service::archive_helper::{ArchiveAction, ArchiveHelper};
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
) -> impl Responder {
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let caching_client = caching_client_data.get_ref().clone();
    let resolver_service = ResolverService::new(ant_tp_config.clone(), caching_client.clone());

    match resolver_service.resolve(&conn.host(), &path.into_inner()).await {
        Some(resolved_address) => {
            let file_service = FileService::new(caching_client.clone(), ant_tp_config.clone());
            let header_builder = HeaderBuilder::new(resolver_service.clone(), ant_tp_config.clone());
            if resolved_address.archive.is_some() {
                debug!("Retrieving file from archive [{:x}]", resolved_address.xor_name);
                let public_archive_service = PublicArchiveService::new(file_service, uploader_state_data, upload_state_data, ant_tp_config.clone(), caching_client);
                let archive_info = public_archive_service.get_archive_info(&resolved_address, &request).await;

                if !archive_info.is_modified {
                    Ok(build_not_modified_response(&resolved_address, &header_builder))
                } else {
                    match archive_info.action {
                        ArchiveAction::Redirect => Ok(build_moved_permanently_response(&request, &header_builder)),
                        ArchiveAction::NotFound => Err(ErrorNotFound(format!("Path not found: [{}]", archive_info.path_string))),
                        ArchiveAction::Listing  => Ok(build_list_files_response(&request, &resolved_address, &header_builder, ant_tp_config)),
                        ArchiveAction::Data => {
                            match public_archive_service.get_data(&request, archive_info, resolved_address.file_path.clone()).await {
                                Ok((chunk_receiver, range_props)) => {
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
                                Err(e) => Err(handle_error(e))
                            }
                        }
                    }
                }
            } else {
                debug!("Retrieving file from XOR [{:x}]", resolved_address.xor_name);
                // todo: could move is_modified to resolved_address and maybe change name?
                if !resolver_service.is_modified(request.headers(), &resolved_address.xor_name) {
                    Ok(build_not_modified_response(&resolved_address, &header_builder))
                } else {
                    match file_service.get_data(&resolved_address, &request, resolved_address.file_path.clone()).await {
                        Ok((chunk_receiver, range_props)) => {
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
                        Err(e) => Err(handle_error(e))
                    }
                }
            }
        },
        None => Err(ErrorNotFound(format!("File not found {:?}", conn.host())))
    }
}

fn build_not_modified_response(resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder) -> HttpResponse {
    HttpResponse::NotModified()
        .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
        .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .finish()
}

fn build_moved_permanently_response(request: &HttpRequest, header_builder: &HeaderBuilder) -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header(header_builder.build_location_header(format!("{}/", request.path())))
        .insert_header(header_builder.build_server_header())
        .finish()
}

fn build_list_files_response(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder,
                             ant_tp_config: AntTpConfig) -> HttpResponse {
    let archive_helper = ArchiveHelper::new(resolved_address.archive.clone().unwrap(), ant_tp_config);
    let mime = archive_helper.get_accept_header_value(request.headers());
    HttpResponse::Ok()
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header_from_mime(mime))
        .body(archive_helper.list_files(resolved_address.file_path.clone(), request.headers()))
}

fn update_partial_content_response(builder: &mut HttpResponseBuilder, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, range_props: &RangeProps) {
    builder
        .insert_header(header_builder.build_content_range_header(range_props.range_from().unwrap(), range_props.range_to().unwrap(), range_props.content_length()))
        .insert_header(header_builder.build_accept_ranges_header())
        .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
        .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header(range_props.extension()));
}

fn update_full_content_response(builder: &mut HttpResponseBuilder, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, range_props: &RangeProps) {
    builder
        .insert_header(header_builder.build_content_length_header(range_props.content_length()))
        .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
        .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header(range_props.extension()));
}

fn handle_error(chunk_error: ChunkError) -> Error {
    match chunk_error {
        ChunkError::GetError(get_error) => handle_get_error(get_error),
        _ => ErrorInternalServerError(chunk_error),
    }
}

