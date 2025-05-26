use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::ClientCacheState;
use crate::config::anttp_config::AntTpConfig;
use crate::service::pointer_service::{Pointer, PointerService};

pub async fn post_pointer(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    pointer: web::Json<Pointer>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let pointer_service = PointerService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new pointer");
    pointer_service.create_pointer(pointer.into_inner(), evm_wallet).await
}

pub async fn put_pointer(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    pointer: web::Json<Pointer>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let pointer_service = PointerService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Updating pointer");
    pointer_service.update_pointer(address, pointer.into_inner()).await
}

pub async fn get_pointer(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let pointer_service = PointerService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting pointer at [{}]", address);
    pointer_service.get_pointer(address).await
}