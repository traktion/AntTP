use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::info;
use crate::client::caching_client::CachingClient;
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
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
) -> impl Responder {
    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new private scratchpad");
    scratchpad_service.create_scratchpad(scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), true).await
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

    info!("Updating private scratchpad");
    scratchpad_service.update_scratchpad(address, scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), true).await
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
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let (address, name) = path.into_inner();

    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting private scratchpad at [{}] with name [{}]", address, name.clone().unwrap_or("".to_string()));
    scratchpad_service.get_scratchpad(address, name, true).await
}