use actix_http::header::HeaderMap;
use actix_web::{web, HttpRequest, HttpResponse, HttpResponseBuilder};
use actix_web::dev::ConnectionInfo;
use actix_web::web::Data;
use log::debug;
use mime::{Mime, APPLICATION_JSON, TEXT_HTML};
use mockall_double::double;
use crate::config::anttp_config::AntTpConfig;
use crate::service::public_archive_service::PublicArchiveService;
#[double]
use crate::client::ChunkCachingClient;
#[double]
use crate::client::PublicArchiveCachingClient;
#[double]
use crate::client::PublicDataCachingClient;
#[double]
use crate::client::CachingClient;
#[double]
use crate::client::StreamingClient;
use crate::error::GetError;
use crate::error::chunk_error::ChunkError;
use crate::service::archive_helper::{ArchiveAction, ArchiveHelper, ArchiveInfo};
#[double]
use crate::service::file_service::FileService;
use crate::service::file_service::{RangeProps};
use crate::service::header_builder::HeaderBuilder;
#[double]
use crate::service::resolver_service::ResolverService;
use crate::service::resolver_service::ResolvedAddress;

pub async fn get_public_data(
    request: HttpRequest,
    path: web::Path<String>,
    resolver_service: Data<ResolverService>,
    caching_client_data: Data<CachingClient>,
    streaming_client_data: Data<StreamingClient>,
    conn: ConnectionInfo,
    ant_tp_config_data: Data<AntTpConfig>,
) -> Result<HttpResponse, ChunkError> {
    fetch_public_data(request, path, resolver_service, caching_client_data, streaming_client_data,
                      conn, ant_tp_config_data, true).await
}

pub async fn head_public_data(
    request: HttpRequest,
    path: web::Path<String>,
    resolver_service: Data<ResolverService>,
    caching_client_data: Data<CachingClient>,
    streaming_client_data: Data<StreamingClient>,
    conn: ConnectionInfo,
    ant_tp_config_data: Data<AntTpConfig>,
) -> Result<HttpResponse, ChunkError> {
    fetch_public_data(request, path, resolver_service, caching_client_data, streaming_client_data,
                      conn, ant_tp_config_data, false).await
}

async fn fetch_public_data(
    request: HttpRequest,
    path: web::Path<String>,
    resolver_service: Data<ResolverService>,
    caching_client_data: Data<CachingClient>,
    streaming_client_data: Data<StreamingClient>,
    conn: ConnectionInfo,
    ant_tp_config_data: Data<AntTpConfig>,
    has_body: bool,
) -> Result<HttpResponse, ChunkError> {
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let caching_client = caching_client_data.get_ref().clone();
    let streaming_client = streaming_client_data.get_ref().clone();

    match resolver_service.resolve(&conn.host(), &path.into_inner(), &request.headers()).await {
        Some(resolved_address) => {
            let header_builder = HeaderBuilder::new(resolved_address.ttl);
            if !resolved_address.is_allowed {
                Err(GetError::AccessNotAllowed(format!("Access forbidden: {}", resolved_address.xor_name)).into())
            } else if !resolved_address.is_modified {
                Ok(build_not_modified_response(&resolved_address, &header_builder))
            } else if resolved_address.archive.is_some() {
                debug!("Retrieving file from archive [{:x}]", resolved_address.xor_name);
                let chunk_caching_client = ChunkCachingClient::new(caching_client.clone());
                let public_archive_caching_client = PublicArchiveCachingClient::new(caching_client.clone(), streaming_client.clone());
                let public_data_caching_client = PublicDataCachingClient::new(caching_client.clone(), streaming_client.clone());
                let file_service = FileService::new(chunk_caching_client, ant_tp_config.download_threads);
                let public_archive_service = PublicArchiveService::new(file_service, public_archive_caching_client, public_data_caching_client, resolver_service.get_ref().clone());
                let archive_info = public_archive_service.get_archive_info(&resolved_address, &request).await;

                match archive_info.action {
                    ArchiveAction::Data => get_data_archive(&request, &resolved_address, &header_builder, public_archive_service, archive_info, has_body).await,
                    ArchiveAction::Redirect => Ok(build_moved_permanently_response(&request.path(), &header_builder)),
                    ArchiveAction::Listing  => Ok(build_list_files_response(&request, &resolved_address, &header_builder, has_body)),
                    ArchiveAction::NotFound => Err(GetError::RecordNotFound(format!("File not found: {}", request.full_url())).into()),
                }
            } else {
                debug!("Retrieving file from XOR [{:x}]", resolved_address.xor_name);
                let chunk_caching_client = ChunkCachingClient::new(caching_client.clone());
                let file_service = FileService::new(chunk_caching_client, ant_tp_config.download_threads);
                get_data_xor(&request, &resolved_address, &header_builder, file_service, has_body).await
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

fn build_list_files_response(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, has_body: bool) -> HttpResponse {
    let archive_helper = ArchiveHelper::new(resolved_address.archive.clone().unwrap());
    let mime = get_accept_header_value(request.headers());
    let body = if has_body {
        archive_helper.list_files(resolved_address.file_path.clone(), request.headers())
    } else {
        "".to_string()
    };

    if mime == APPLICATION_JSON {
        HttpResponse::Ok()
            .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
            .insert_header(header_builder.build_cors_header())
            .insert_header(header_builder.build_server_header())
            .insert_header(header_builder.build_content_type_header_from_mime(&mime))
            .body(body)
    } else {
        HttpResponse::Ok()
            // can only use etag for one content-type currently. JSON can have priority as could cause app issues.
            .insert_header(header_builder.build_cors_header())
            .insert_header(header_builder.build_server_header())
            .insert_header(header_builder.build_content_type_header_from_mime(&mime))
            .body(body)
    }
}

fn update_partial_content_response(builder: &mut HttpResponseBuilder, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, range_props: &RangeProps, modified_time: Option<u64>) {
    builder
        .insert_header(header_builder.build_content_range_header(range_props.range_from().unwrap(), range_props.range_to().unwrap(), range_props.content_length()))
        .insert_header(header_builder.build_accept_ranges_header())
        .insert_header(header_builder.build_cache_control_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_expires_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header(range_props.extension()));
    if modified_time.is_some() {
        builder.insert_header(header_builder.build_last_modified_header(modified_time.unwrap()));
    }
}

fn update_full_content_response(builder: &mut HttpResponseBuilder, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, range_props: &RangeProps, modified_time: Option<u64>) {
    builder
        .insert_header(header_builder.build_content_length_header(range_props.content_length()))
        .insert_header(header_builder.build_cache_control_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_expires_header(resolved_address.is_resolved_from_mutable))
        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
        .insert_header(header_builder.build_cors_header())
        .insert_header(header_builder.build_server_header())
        .insert_header(header_builder.build_content_type_header(range_props.extension()));
    if modified_time.is_some() {
        builder.insert_header(header_builder.build_last_modified_header(modified_time.unwrap()));
    }
}

async fn get_data_archive(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, public_archive_service: PublicArchiveService, archive_info: ArchiveInfo, has_body: bool) -> Result<HttpResponse, ChunkError> {
    let (chunk_receiver, range_props) = public_archive_service.get_data(&request, archive_info.clone()).await?;
    if range_props.is_range() {
        let mut builder = HttpResponse::PartialContent();
        update_partial_content_response(&mut builder, &resolved_address, &header_builder, &range_props, Some(archive_info.modified_time));
        if has_body {
            Ok(builder.streaming(chunk_receiver))
        } else {
            Ok(builder.no_chunking(range_props.content_length()).streaming(chunk_receiver))
        }
    } else {
        let mut builder = HttpResponse::Ok();
        update_full_content_response(&mut builder, &resolved_address, &header_builder, &range_props, Some(archive_info.modified_time));
        if has_body {
            Ok(builder.streaming(chunk_receiver))
        } else {
            Ok(builder.no_chunking(range_props.content_length()).streaming(chunk_receiver))
        }
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

async fn get_data_xor(request: &HttpRequest, resolved_address: &ResolvedAddress, header_builder: &HeaderBuilder, file_service: FileService, has_body: bool) -> Result<HttpResponse, ChunkError> {
    let (chunk_receiver, range_props) = file_service.get_data(&request, &resolved_address).await?;
    if range_props.is_range() {
        let mut builder = HttpResponse::PartialContent();
        update_partial_content_response(&mut builder, &resolved_address, &header_builder, &range_props, None);
        if has_body {
            Ok(builder.streaming(chunk_receiver))
        } else {
            Ok(builder.no_chunking(range_props.content_length()).streaming(chunk_receiver))
        }
    } else {
        let mut builder = HttpResponse::Ok();
        update_full_content_response(&mut builder, &resolved_address, &header_builder, &range_props, None);
        if has_body {
            Ok(builder.streaming(chunk_receiver))
        } else {
            Ok(builder.no_chunking(range_props.content_length()).streaming(chunk_receiver))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::{TestRequest};
    use crate::client::client_harness::ClientHarness;
    use ant_evm::EvmNetwork;
    use foyer::HybridCacheBuilder;
    use tokio::sync::mpsc;
    use crate::client::command::Command;
    use crate::service::access_checker::AccessChecker;
    use crate::service::bookmark_resolver::BookmarkResolver;
    use crate::service::pointer_name_resolver::PointerNameResolver;
    use crate::client::MockPointerCachingClient;
    use crate::client::MockChunkCachingClient;
    use crate::client::MockStreamingClient;
    use crate::client::MockCachingClient;
    use autonomi::SecretKey;
    use clap::Parser;

    use crate::error::pointer_error::PointerError;
    use crate::error::GetError;
    use crate::service::resolver_service::MockResolverService;

    async fn create_test_data() -> (Data<ResolverService>, Data<CachingClient>, Data<MockStreamingClient>, Data<AntTpConfig>) {
        let config = AntTpConfig::parse_from(vec!["anttp"]);
        let evm_network = EvmNetwork::ArbitrumOne;
        let client_harness = Data::new(tokio::sync::Mutex::new(ClientHarness::new(evm_network, config.clone())));
        let hybrid_cache = Data::new(HybridCacheBuilder::new().memory(10).storage().build().await.unwrap());
        let (tx, _rx) = mpsc::channel::<Box<dyn Command>>(100);
        let command_executor = Data::new(tx);
        let mut mock_caching_client = MockCachingClient::default();
        mock_caching_client.expect_clone().returning(MockCachingClient::default);

        let hc = hybrid_cache.clone();
        let ctx = MockCachingClient::new_context();
        ctx.expect()
            .returning(move |client_harness, config, hybrid_cache, command_executor| {
                let mut mock = MockCachingClient::default();
                mock.expect_get_hybrid_cache().return_const(hc.clone());
                let hc_for_clone = hc.clone();
                mock.expect_clone().returning(move || {
                    let mut m = MockCachingClient::default();
                    m.expect_get_hybrid_cache().return_const(hc_for_clone.clone());
                    m.expect_clone().returning(MockCachingClient::default);
                    m
                });
                mock
            });

        let caching_client = Data::new(CachingClient::new(client_harness, config.clone(), hybrid_cache, command_executor));
        
        let access_checker = Data::new(tokio::sync::Mutex::new(AccessChecker::new()));
        let bookmark_resolver = Data::new(tokio::sync::Mutex::new(BookmarkResolver::new()));
        
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        mock_pointer_caching_client
            .expect_clone()
            .returning(|| {
                let mut m = MockPointerCachingClient::default();
                m.expect_pointer_get()
                    .returning(|_| Err(PointerError::GetError(GetError::RecordNotFound("Not found".to_string()))));
                m
            });
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(|_| Err(PointerError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let mock_chunk_caching_client = MockChunkCachingClient::default();

        let pointer_name_resolver = Data::new(PointerNameResolver::new(
            mock_pointer_caching_client.clone(),
            mock_chunk_caching_client,
            SecretKey::default(),
            1,
        ));

        let mut mock_streaming_client = MockStreamingClient::default();
        mock_streaming_client.expect_clone().returning(MockStreamingClient::default);

        let mut mock_resolver = MockResolverService::default();
        mock_resolver.expect_resolve().returning(|_, _, _| None);

        (Data::new(mock_resolver), caching_client, Data::new(mock_streaming_client), Data::new(config))
    }

    #[actix_web::test]
    async fn test_get_public_data_not_found() {
        let (resolver_service, caching_client, streaming_client, config) = create_test_data().await;
        let req = TestRequest::get().uri("/nonexistent").to_http_request();
        let path = web::Path::from("nonexistent".to_string());
        let conn = req.connection_info().clone();
        
        let result = get_public_data(req, path, resolver_service, caching_client, streaming_client, conn, config).await;
        assert!(result.is_err());
    }

    #[actix_web::test]
    async fn test_head_public_data_not_found() {
        let (resolver_service, caching_client, streaming_client, config) = create_test_data().await;
        let req = TestRequest::default().method(actix_web::http::Method::HEAD).uri("/nonexistent").to_http_request();
        let path = web::Path::from("nonexistent".to_string());
        let conn = req.connection_info().clone();
        
        let result = head_public_data(req, path, resolver_service, caching_client, streaming_client, conn, config).await;
        assert!(result.is_err());
    }
}


