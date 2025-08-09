use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::service::pointer_service::{Pointer, PointerService};

#[utoipa::path(
    post,
    path = "/anttp-0/pointer",
    request_body(
        content = Pointer
    ),
    responses(
        (status = CREATED, description = "Pointer created successfully", body = Pointer)
    ),
)]
pub async fn post_pointer(
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    pointer: web::Json<Pointer>,
) -> impl Responder {    
    let pointer_service = PointerService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new pointer");
    pointer_service.create_pointer(pointer.into_inner(), evm_wallet_data.get_ref().clone()).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/pointer/{address}",
    params(
        ("address", description = "Address of pointer")
    ),
    request_body(
        content = Pointer
    ),
    responses(
        (status = OK, description = "Pointer updated successfully", body = Pointer)
    ),
)]
pub async fn put_pointer(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    pointer: web::Json<Pointer>,
) -> impl Responder {
    let address = path.into_inner();

    let pointer_service = PointerService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Updating pointer");
    pointer_service.update_pointer(address, pointer.into_inner()).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/pointer/{address}",
        params(
        ("address" = String, Path, description = "Pointer address"),
    ),
    responses(
        (status = OK, description = "Pointer found successfully", body = Pointer),
        (status = NOT_FOUND, description = "Pointer was not found")
    ),
    params(
        ("address" = String, Path, description = "Pointer address"),
    )
)]
pub async fn get_pointer(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let address = path.into_inner();

    let pointer_service = PointerService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting pointer at [{}]", address);
    pointer_service.get_pointer(address).await
}