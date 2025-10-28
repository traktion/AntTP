use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::client::CachingClient;
use crate::error::scratchpad_error::ScratchpadError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::cache_only;
use crate::service::scratchpad_service::{Scratchpad, ScratchpadService};

#[utoipa::path(
    post,
    path = "/anttp-0/public_scratchpad/{name}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = CREATED, description = "Public scratchpad created successfully", body = Scratchpad)
    ),
    params(
        ("name" = String, Path, description = "Public scratchpad name"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_scratchpad(
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

    debug!("Creating new public scratchpad");
    scratchpad_service.create_scratchpad(name, scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), false, cache_only(request)).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/public_scratchpad/{address}/{name}",
    request_body(
        content = Scratchpad
    ),
    responses(
        (status = OK, description = "Public scratchpad updated successfully", body = Scratchpad)
    ),
    params(
        ("address" = String, Path, description = "Public scratchpad address"),
        ("name" = String, Path, description = "Public scratchpad name"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_public_scratchpad(
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

    debug!("Updating public scratchpad");
    scratchpad_service.update_scratchpad(address, name, scratchpad.into_inner(), evm_wallet_data.get_ref().clone(), false, cache_only(request)).await
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
) -> Result<HttpResponse, ScratchpadError> {
    let address = path.into_inner();

    let scratchpad_service = ScratchpadService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone(),
    );

    debug!("Getting public scratchpad at [{}]", address);
    Ok(HttpResponse::Ok().json(scratchpad_service.get_scratchpad(address, None, false).await?))
}