use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use actix_web::dev::ConnectionInfo;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::web::Data;
use log::{debug, info};
use crate::config::anttp_config::AntTpConfig;
use crate::{UploaderState, UploadState};
use crate::service::public_archive_service::PublicArchiveService;
use crate::client::CachingClient;
use crate::client::error::ChunkError;
use crate::controller::handle_get_error;
use crate::service::archive_helper::{ArchiveAction, ArchiveHelper, DataState};
use crate::service::file_service::FileService;
use crate::service::header_builder::HeaderBuilder;
use crate::service::resolver_service::ResolverService;

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
    let path_parts = get_path_parts(&conn.host(), &path.into_inner(), ant_tp_config.clone(), caching_client.clone());
    let (archive_addr, archive_file_name) = resolver_service.assign_path_parts(path_parts.clone());

    // todo: refactor to break this down and avoid duplicate code
    match resolver_service.resolve_archive_or_file(&archive_addr, &archive_file_name, false).await {
        Some(resolved_address) => {
            let file_service = FileService::new(caching_client.clone(), ant_tp_config.clone());
            let header_builder = HeaderBuilder::new(resolver_service.clone(), ant_tp_config.clone());
            if resolved_address.archive.is_some() {
                debug!("Retrieving file from archive [{:x}]", resolved_address.xor_name);
                let public_archive_service = PublicArchiveService::new(file_service, resolver_service, uploader_state_data, upload_state_data, ant_tp_config.clone(), caching_client);
                let archive_info = public_archive_service.get_archive_info(&resolved_address, &request, &path_parts).await;

                if archive_info.state == DataState::NotModified {
                    debug!("ETag matches for path [{}] at address [{}]. Client can use cached version", archive_info.path_string, format!("{:x}", archive_info.resolved_xor_addr));
                    Ok(HttpResponse::NotModified()
                        .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
                        .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
                        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                        .insert_header(header_builder.build_cors_header())
                        .insert_header(header_builder.build_server_header())
                        .finish())
                } else if archive_info.action == ArchiveAction::Redirect {
                    Ok(HttpResponse::MovedPermanently()
                        .insert_header(header_builder.build_location_header(format!("{}/", request.path())))
                        .insert_header(header_builder.build_server_header())
                        .finish())
                } else if archive_info.action == ArchiveAction::NotFound {
                    Err(ErrorNotFound(format!("Path not found: [{}]", archive_info.path_string)))
                } else if archive_info.action == ArchiveAction::Listing {
                    let archive_helper = ArchiveHelper::new(resolved_address.archive.unwrap(), ant_tp_config);
                    let archive_relative_path = path_parts[1..].join("/").to_string();
                    debug!("List files in archive at path: [{}]", archive_relative_path);
                    let mime = archive_helper.get_accept_header_value(request.headers());
                    Ok(HttpResponse::Ok()
                        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                        .insert_header(header_builder.build_cors_header())
                        .insert_header(header_builder.build_server_header())
                        .insert_header(header_builder.build_content_type_header_from_mime(mime))
                        .body(archive_helper.list_files(archive_relative_path, request.headers()))) // todo: return .json / .body depending on accept header
                } else {
                    match public_archive_service.get_data(request, path_parts, archive_info).await {
                        Ok((chunk_receiver, range_props)) => {
                            if range_props.is_range() {
                                Ok(HttpResponse::PartialContent()
                                    .insert_header(header_builder.build_content_range_header(range_props.range_from().unwrap(), range_props.range_to().unwrap(), range_props.content_length()))
                                    .insert_header(header_builder.build_accept_ranges_header())
                                    .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                                    .insert_header(header_builder.build_cors_header())
                                    .insert_header(header_builder.build_server_header())
                                    .insert_header(header_builder.build_content_type_header(range_props.extension()))
                                    .streaming(chunk_receiver))
                            } else {
                                Ok(HttpResponse::Ok()
                                    .insert_header(header_builder.build_content_length_header(range_props.content_length()))
                                    .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                                    .insert_header(header_builder.build_cors_header())
                                    .insert_header(header_builder.build_server_header())
                                    .insert_header(header_builder.build_content_type_header(range_props.extension()))
                                    .streaming(chunk_receiver))
                            }
                        }
                        Err(e) => Err(handle_error(e))
                    }
                }
            } else {
                debug!("Retrieving file from XOR [{:x}]", resolved_address.xor_name);

                if resolver_service.get_data_state(request.headers(), &resolved_address.xor_name) == DataState::NotModified {
                    let (archive_addr, _) = resolver_service.assign_path_parts(path_parts.clone());
                    info!("ETag matches for path [{}] at address [{}]. Client can use cached version", archive_addr, format!("{:x}", resolved_address.xor_name).as_str());

                    Ok(HttpResponse::NotModified()
                        .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
                        .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
                        .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                        .insert_header(header_builder.build_cors_header())
                        .insert_header(header_builder.build_server_header())
                        .finish())
                } else {
                    match file_service.get_data(&resolved_address, &request, &path_parts).await {
                        Ok((chunk_receiver, range_props)) => {
                            if range_props.is_range() {
                                Ok(HttpResponse::PartialContent()
                                    .insert_header(header_builder.build_content_range_header(range_props.range_from().unwrap(), range_props.range_to().unwrap(), range_props.content_length()))
                                    .insert_header(header_builder.build_accept_ranges_header())
                                    .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                                    .insert_header(header_builder.build_cors_header())
                                    .insert_header(header_builder.build_server_header())
                                    .insert_header(header_builder.build_content_type_header(range_props.extension()))
                                    .streaming(chunk_receiver))
                            } else {
                                Ok(HttpResponse::Ok()
                                    .insert_header(header_builder.build_content_length_header(range_props.content_length()))
                                    .insert_header(header_builder.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable))
                                    .insert_header(header_builder.build_etag_header(&resolved_address.xor_name))
                                    .insert_header(header_builder.build_cors_header())
                                    .insert_header(header_builder.build_server_header())
                                    .insert_header(header_builder.build_content_type_header(range_props.extension()))
                                    .streaming(chunk_receiver))
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

fn get_path_parts(hostname: &str, path: &str, ant_tp_config: AntTpConfig, caching_client: CachingClient) -> Vec<String> {
    let xor_helper = ResolverService::new(ant_tp_config.clone(), caching_client.clone());
    // assert: subdomain.autonomi as acceptable format
    if hostname.ends_with(".autonomi") {
        let mut subdomain_parts = hostname.split(".")
            .map(str::to_string)
            .collect::<Vec<String>>();
        subdomain_parts.pop(); // discard 'autonomi' suffix
        let path_parts = path.split("/")
            .map(str::to_string)
            .collect::<Vec<String>>();
        subdomain_parts.append(&mut path_parts.clone());
        subdomain_parts
    } else if xor_helper.is_valid_hostname(&hostname.to_string()) {
        let mut subdomain_parts = Vec::new();
        subdomain_parts.push(hostname.to_string());
        let path_parts = path.split("/")
            .map(str::to_string)
            .collect::<Vec<String>>();
        subdomain_parts.append(&mut path_parts.clone());
        subdomain_parts
    } else {
        let path_parts = path.split("/")
            .map(str::to_string)
            .collect::<Vec<String>>();
        path_parts.clone()
    }
}

fn handle_error(chunk_error: ChunkError) -> Error {
    match chunk_error {
        ChunkError::GetError(get_error) => handle_get_error(get_error),
        _ => ErrorInternalServerError(chunk_error),
    }
}

