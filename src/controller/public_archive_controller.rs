use actix_multipart::Multipart;
use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use foyer::HybridCache;
use log::info;
use crate::config::anttp_config::AntTpConfig;
use crate::{UploaderState, ClientCacheState, UploadState};
use crate::service::public_archive_service::{PublicArchiveService, Upload};
use crate::client::caching_client::CachingClient;
use crate::service::file_service::FileService;
use crate::service::resolver_service::ResolverService;

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/public_archive",
    request_body(
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Public archive created successfully", body = Upload)
    ),
)]
pub async fn post_public_archive(
    payload: Multipart,
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    client_cache_state: Data<ClientCacheState>,
    ant_tp_config: Data<AntTpConfig>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
)
    -> impl Responder {
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), uploader_state, upload_state, ant_tp_config.clone(), client_cache_state, hybrid_cache_data);
    let evm_wallet = evm_wallet_data.get_ref().clone();

    info!("Creating new archive from multipart POST");
    archive_service.create_public_archive(payload, evm_wallet).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/multipart/public_archive",
    request_body(
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Public archive updated successfully", body = Upload)
    ),
)]
pub async fn put_public_archive(
    path: web::Path<String>,
    payload: Multipart,
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    client_cache_state: Data<ClientCacheState>,
    ant_tp_config: Data<AntTpConfig>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
)
    -> impl Responder {
    let address = path.into_inner();
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), uploader_state, upload_state, ant_tp_config.clone(), client_cache_state, hybrid_cache_data);
    let evm_wallet = evm_wallet_data.get_ref().clone();

    info!("Updating [{}] archive from multipart PUT", address);
    archive_service.update_public_archive(address, payload, evm_wallet).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/public_archive/status/{id}",
    responses(
        (status = 200, description = "Id found successfully", body = Upload),
        (status = NOT_FOUND, description = "Id was not found")
    ),
    params(
        ("id" = String, Path, description = "Id of upload"),
    )
)]
pub async fn get_status_public_archive(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    ant_tp_config: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let id = path.into_inner();
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), uploader_state, upload_state, ant_tp_config.clone(), client_cache_state, hybrid_cache_data);

    info!("Checking upload status for [{:?}]", id);
    archive_service.get_status(id).await
}

fn build_archive_service(autonomi_client: Client, uploader_state: Data<UploaderState>, upload_state: Data<UploadState>, ant_tp_config_data: Data<AntTpConfig>, client_cache_state: Data<ClientCacheState>, hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>) -> PublicArchiveService {
    let ant_tp_config = ant_tp_config_data.get_ref();
    let caching_client = CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data);
    let resolver_service = ResolverService::new(ant_tp_config.clone(), caching_client.clone());
    let file_service = FileService::new(caching_client.clone(), resolver_service.clone(), ant_tp_config.clone());
    PublicArchiveService::new(autonomi_client, file_service, resolver_service, uploader_state, upload_state, ant_tp_config.clone(), caching_client)
}