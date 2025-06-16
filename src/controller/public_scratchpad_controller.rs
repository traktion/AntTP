use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::ClientCacheState;
use crate::config::anttp_config::AntTpConfig;
use crate::service::scratchpad_service::{Scratchpad, ScratchpadService};

#[utoipa::path(
    post,
    path = "/api/v1/public_scratchpad",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = CREATED, description = "Public scratchpad created successfully", body = Scratchpad)
    ),
)]
pub async fn post_public_scratchpad(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let scratchpad_service = ScratchpadService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new public scratchpad");
    scratchpad_service.create_scratchpad(scratchpad.into_inner(), evm_wallet, false).await
}

#[utoipa::path(
    put,
    path = "/api/v1/public_scratchpad/{address}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = OK, description = "Public scratchpad updated successfully", body = Scratchpad)
    ),
)]
pub async fn put_public_scratchpad(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let address = path.into_inner();

    let evm_wallet = evm_wallet_data.get_ref().clone();
    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let scratchpad_service = ScratchpadService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Updating public scratchpad");
    scratchpad_service.update_scratchpad(address, scratchpad.into_inner(), evm_wallet, false).await
}

#[utoipa::path(
    get,
    path = "/api/v1/public_scratchpad/{address}",
    responses(
        (status = OK, description = "Public scratchpad found successfully", body = Scratchpad),
        (status = NOT_FOUND, description = "Public scratchpad was not found")
    ),
    params(
        ("address" = String, Path, description = "Public scratchpad address"),
    )
)]
pub async fn get_public_scratchpad(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let scratchpad_service = ScratchpadService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting public scratchpad at [{}]", address);
    scratchpad_service.get_scratchpad(address, None, false).await
}