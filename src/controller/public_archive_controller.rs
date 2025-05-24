use actix_multipart::Multipart;
use actix_web::dev::ConnectionInfo;
use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::anttp_config::AntTpConfig;
use crate::AppState;
use crate::service::public_archive_service::PublicArchiveService;
use crate::caching_client::CachingClient;
use crate::service::file_service::FileService;
use crate::service::resolver_service::ResolverService;

pub async fn post_public_archive(
    payload: Multipart,
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    conn: ConnectionInfo,
    app_state: Data<AppState>,
    ant_tp_config: Data<AntTpConfig>,
)
    -> impl Responder {
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), conn, app_state, ant_tp_config.clone());
    let evm_wallet = evm_wallet_data.get_ref().clone();

    info!("Creating new archive from multipart POST");
    archive_service.post_data(payload, evm_wallet).await
}

pub async fn get_status_public_archive(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    conn: ConnectionInfo,
    app_state: Data<AppState>,
    ant_tp_config: Data<AntTpConfig>,
) -> impl Responder {
    let id = path.into_inner();
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), conn, app_state, ant_tp_config.clone());

    info!("Checking upload status for [{:?}]", id);
    archive_service.get_status(id).await
}

fn build_archive_service(autonomi_client: Client, conn: ConnectionInfo, app_state: Data<AppState>, ant_tp_config_data: Data<AntTpConfig>) -> PublicArchiveService {
    let ant_tp_config = ant_tp_config_data.get_ref();
    let caching_autonomi_client = CachingClient::new(autonomi_client.clone(), ant_tp_config.clone());
    let xor_helper = ResolverService::new(ant_tp_config.clone());
    let file_service = FileService::new(autonomi_client.clone(), xor_helper.clone(), conn, ant_tp_config.clone());
    PublicArchiveService::new(autonomi_client, caching_autonomi_client, file_service, xor_helper, app_state, ant_tp_config.clone())
}