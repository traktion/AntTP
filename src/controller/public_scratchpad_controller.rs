use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::service::scratchpad_service::{Scratchpad, ScratchpadService};

#[utoipa::path(
    post,
    path = "/anttp-0/public_scratchpad",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = CREATED, description = "Public scratchpad created successfully", body = Scratchpad)
    ),
)]
pub async fn post_public_scratchpad(
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
) -> impl Responder {
    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new public scratchpad");
    scratchpad_service.create_scratchpad(scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), false).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/public_scratchpad/{address}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = OK, description = "Public scratchpad updated successfully", body = Scratchpad)
    ),
)]
pub async fn put_public_scratchpad(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
) -> impl Responder {
    let address = path.into_inner();

    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Updating public scratchpad");
    scratchpad_service.update_scratchpad(address, scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), false).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/public_scratchpad/{address}",
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
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let address = path.into_inner();

    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting public scratchpad at [{}]", address);
    scratchpad_service.get_scratchpad(address, None, false).await
}