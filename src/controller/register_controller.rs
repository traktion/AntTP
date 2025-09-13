use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::info;
use crate::client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::service::register_service::{Register, RegisterService};

#[utoipa::path(
    post,
    path = "/anttp-0/register",
    request_body(
        content = Register
    ),
    responses(
        (status = CREATED, description = "Register created successfully", body = Register)
    ),
)]
pub async fn post_register(
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>,
) -> impl Responder {
    let register_service = RegisterService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new register");
    register_service.create_register(register.into_inner(), evm_wallet_data.get_ref().clone()).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/register/{address}",
    params(
        ("address", description = "Address of pointer")
    ),
    request_body(
        content = Register
    ),
    responses(
        (status = OK, description = "Register updated successfully", body = Register)
    ),
)]
pub async fn put_register(
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    path: web::Path<String>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>,
) -> impl Responder {
    let address = path.into_inner();

    let register_service = RegisterService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Updating register");
    register_service.update_register(address, register.into_inner(), evm_wallet_data.get_ref().clone()).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/register/{address}",
    responses(
        (status = OK, description = "Register found successfully", body = Register),
        (status = NOT_FOUND, description = "Register was not found")
    ),
    params(
        ("address" = String, Path, description = "Register address"),
    )
)]
pub async fn get_register(
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();

    let register_service = RegisterService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting register at [{}]", address);
    register_service.get_register(address).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/register_history/{address}",
    responses(
        (status = OK, description = "Register history found successfully", body = [Register]),
        (status = NOT_FOUND, description = "Register history was not found")
    ),
    params(
        ("address" = String, Path, description = "Register address"),
    )
)]
pub async fn get_register_history(
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();

    let register_service = RegisterService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting register history at [{}]", address);
    register_service.get_register_history(address).await
}