use actix_web::{web, HttpRequest, Responder};
use actix_web::dev::ConnectionInfo;
use actix_web::error::ErrorNotFound;
use actix_web::web::Data;
use log::debug;
use crate::config::anttp_config::AntTpConfig;
use crate::{UploaderState, UploadState};
use crate::service::public_archive_service::PublicArchiveService;
use crate::client::CachingClient;
use crate::service::file_service::FileService;
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

    match resolver_service.resolve_archive_or_file(&archive_addr, &archive_file_name, false).await {
        Some(resolved_address) => {
            let file_service = FileService::new(caching_client.clone(), resolver_service.clone(), ant_tp_config.clone());
            if resolved_address.archive.is_some() {
                debug!("Retrieving file from archive [{:x}]", resolved_address.xor_name);
                let public_archive_service = PublicArchiveService::new(file_service, resolver_service, uploader_state_data, upload_state_data, ant_tp_config, caching_client);
                public_archive_service.get_data(resolved_address, request, path_parts).await
            } else {
                debug!("Retrieving file from XOR [{:x}]", resolved_address.xor_name);
                file_service.get_data(resolved_address, request, path_parts).await
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