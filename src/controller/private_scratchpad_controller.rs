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
    path = "/anttp-0/private_scratchpad",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = CREATED, description = "Private scratchpad created successfully", body = Scratchpad)
    ),
)]
pub async fn post_private_scratchpad(
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

    info!("Creating new private scratchpad");
    scratchpad_service.create_scratchpad(scratchpad.into_inner(), evm_wallet, true).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/private_scratchpad/{address}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = OK, description = "Private scratchpad updated successfully", body = Scratchpad)
    ),
)]
pub async fn put_private_scratchpad(
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

    info!("Updating private scratchpad");
    scratchpad_service.update_scratchpad(address, scratchpad.into_inner(), evm_wallet, true).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/private_scratchpad/{address}/{name}",
    responses(
        (status = OK, description = "Private scratchpad found successfully", body = Scratchpad),
        (status = NOT_FOUND, description = "Private scratchpad was not found")
    ),
    params(
        ("address" = String, Path, description = "Private scratchpad address"),
        ("name" = String, Path, description = "Private scratchpad name"),
    )
)]
pub async fn get_private_scratchpad(
    path: web::Path<(String, Option<String>)>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let (address, name) = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let scratchpad_service = ScratchpadService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting private scratchpad at [{}] with name [{}]", address, name.clone().unwrap_or("".to_string()));
    scratchpad_service.get_scratchpad(address, name, true).await
}