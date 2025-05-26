use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::ClientCacheState;
use crate::config::anttp_config::AntTpConfig;
use crate::service::register_service::{Register, RegisterService};

pub async fn post_register(
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let register_service = RegisterService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config.clone(),
    );

    info!("Creating new register");
    register_service.create_register(register.into_inner(), evm_wallet).await
}

pub async fn put_register(
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    path: web::Path<String>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let register_service = RegisterService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config.clone(),
    );

    info!("Updating register");
    register_service.update_register(address, register.into_inner(), evm_wallet).await
}

pub async fn get_register(
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let register_service = RegisterService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config.clone(),
    );

    info!("Getting register at [{}]", address);
    register_service.get_register(address).await
}

pub async fn get_register_history(
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let register_service = RegisterService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state),
        ant_tp_config.clone(),
    );

    info!("Getting register history at [{}]", address);
    register_service.get_register_history(address).await
}