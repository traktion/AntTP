use actix_web::{web, HttpRequest, Responder};
use actix_web::dev::ConnectionInfo;
use actix_web::web::Data;
use autonomi::Client;
use log::info;
use crate::config::anttp_config::AntTpConfig;
use crate::AppState;
use crate::service::public_archive_service::PublicArchiveService;
use crate::client::caching_client::CachingClient;
use crate::service::file_service::FileService;
use crate::service::resolver_service::ResolverService;

pub async fn get_public_data(
    request: HttpRequest,
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    conn: ConnectionInfo,
    app_state: Data<AppState>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let path_parts = get_path_parts(&conn.host(), &path.into_inner(), ant_tp_config.clone());
    let xor_helper = ResolverService::new(ant_tp_config.clone());
    let (archive_addr, archive_file_name) = xor_helper.assign_path_parts(path_parts.clone());

    let autonomi_client = autonomi_client_data.get_ref().clone();
    let caching_autonomi_client = CachingClient::new(autonomi_client.clone(), ant_tp_config.clone());

    //let (is_found, archive, is_archive, xor_addr) = xor_helper.resolve_archive_or_file(autonomi_client.clone(), &caching_autonomi_client, &archive_addr, &archive_file_name).await;
    let resolved_address = xor_helper.resolve_archive_or_file(autonomi_client.clone(), &caching_autonomi_client, &archive_addr, &archive_file_name).await;
    let file_service = FileService::new(autonomi_client.clone(), xor_helper.clone(), conn, ant_tp_config.clone());

    if !resolved_address.is_archive {
        info!("Retrieving file from XOR [{:x}]", resolved_address.xor_addr);
        file_service.get_data(path_parts, request, resolved_address.xor_addr, resolved_address.is_found).await
    } else {
        info!("Retrieving file from public archive [{:x}]", resolved_address.xor_addr);
        let public_archive_service = PublicArchiveService::new(autonomi_client, caching_autonomi_client.clone(), file_service, xor_helper, app_state, ant_tp_config);
        public_archive_service.get_data(resolved_address.archive, resolved_address.xor_addr, request, path_parts).await
    }
}

fn get_path_parts(hostname: &str, path: &str, ant_tp_config: AntTpConfig) -> Vec<String> {
    let xor_helper = ResolverService::new(ant_tp_config.clone());
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