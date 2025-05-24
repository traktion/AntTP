use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::config::anttp_config::AntTpConfig;
use crate::service::register_service::{Register, RegisterService};

pub async fn post_register(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config: Data<AntTpConfig>,
    register: web::Json<Register>
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let register_service = RegisterService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Creating new register");
    register_service.create_register(register.into_inner(), evm_wallet).await
}

pub async fn put_register(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config: Data<AntTpConfig>,
    register: web::Json<Register>
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();
    let address = path.into_inner();

    let register_service = RegisterService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Updating register");
    register_service.update_register(address, register.into_inner(), evm_wallet).await
}

pub async fn get_register(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config: Data<AntTpConfig>,
) -> impl Responder {
    let address = path.into_inner();

    let register_service = RegisterService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Getting register at [{}]", address);
    register_service.get_register(address).await
}

pub async fn get_register_history(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config: Data<AntTpConfig>,
) -> impl Responder {
    let address = path.into_inner();

    let register_service = RegisterService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Getting register history at [{}]", address);
    register_service.get_register_history(address).await
}