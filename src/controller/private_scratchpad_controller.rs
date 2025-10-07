use actix_web::{web, HttpRequest, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::info;
use crate::client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::cache_only;
use crate::service::scratchpad_service::{Scratchpad, ScratchpadService};

#[utoipa::path(
    post,
    path = "/anttp-0/private_scratchpad/{name}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = CREATED, description = "Private scratchpad created successfully", body = Scratchpad)
    ),
    params(
        ("name" = String, Path, description = "Private scratchpad name"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_private_scratchpad(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
    request: HttpRequest,
) -> impl Responder {
    let name = path.into_inner();
    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new private scratchpad");
    scratchpad_service.create_scratchpad(name, scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), true, cache_only(request)).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/private_scratchpad/{address}/{name}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = OK, description = "Private scratchpad updated successfully", body = Scratchpad)
    ),
    params(
        ("address" = String, Path, description = "Private scratchpad address"),
        ("name" = String, Path, description = "Private scratchpad name"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_private_scratchpad(
    path: web::Path<(String, String)>,
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    scratchpad: web::Json<Scratchpad>,
    request: HttpRequest,
) -> impl Responder {
    let (address, name) = path.into_inner();

    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Updating private scratchpad");
    scratchpad_service.update_scratchpad(address, name, scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), true, cache_only(request)).await
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
    path: web::Path<(String, String)>,
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let (address, name) = path.into_inner();

    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting private scratchpad at [{}] with name [{}]", address, name);
    scratchpad_service.get_scratchpad(address, Some(name), true).await
}