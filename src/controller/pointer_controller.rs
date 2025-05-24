use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::config::anttp_config::AntTpConfig;
use crate::service::pointer_service::{Pointer, PointerService};

pub async fn post_pointer(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config: Data<AntTpConfig>,
    pointer: web::Json<Pointer>
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let pointer_service = PointerService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Creating new pointer");
    pointer_service.create_pointer(pointer.into_inner(), evm_wallet).await
}

pub async fn put_pointer(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config: Data<AntTpConfig>,
    pointer: web::Json<Pointer>
) -> impl Responder {
    let address = path.into_inner();

    let pointer_service = PointerService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Updating pointer");
    pointer_service.update_pointer(address, pointer.into_inner()).await
}

pub async fn get_pointer(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config: Data<AntTpConfig>,
) -> impl Responder {
    let address = path.into_inner();

    let pointer_service = PointerService::new(
        autonomi_client_data.get_ref().clone(),
        ant_tp_config.get_ref().clone(),
    );

    info!("Getting pointer at [{}]", address);
    pointer_service.get_pointer(address).await
}