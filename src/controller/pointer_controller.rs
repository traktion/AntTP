use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::client::CachingClient;
use crate::error::pointer_error::PointerError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::cache_only;
use crate::service::pointer_service::{Pointer, PointerService};
use crate::service::resolver_service::ResolverService;

#[utoipa::path(
    post,
    path = "/anttp-0/pointer",
    request_body(
        content = Pointer
    ),
    responses(
        (status = CREATED, description = "Pointer created successfully", body = Pointer)
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_pointer(
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    pointer: web::Json<Pointer>,
    request: HttpRequest,
) -> impl Responder {
    let pointer_service = create_pointer_service(caching_client_data, ant_tp_config_data);

    debug!("Creating new pointer");
    pointer_service.create_pointer(pointer.into_inner(), evm_wallet_data.get_ref().clone(), cache_only(request)).await
}

#[utoipa::path(
    put,
    path = "/anttp-0/pointer/{address}",
    params(
        ("address", description = "Address of pointer"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (true|false)",
        example = "true")
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
    request: HttpRequest,
) -> impl Responder {
    let address = path.into_inner();

    let pointer_service = create_pointer_service(caching_client_data, ant_tp_config_data);

    debug!("Updating pointer");
    pointer_service.update_pointer(address, pointer.into_inner(), cache_only(request)).await
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
) -> Result<HttpResponse, PointerError> {
    let address = path.into_inner();

    let pointer_service = create_pointer_service(caching_client_data, ant_tp_config_data);

    debug!("Getting pointer at [{}]", address);
    Ok(HttpResponse::Ok().json(pointer_service.get_pointer(address).await?))
}

fn create_pointer_service(caching_client_data: Data<CachingClient>, ant_tp_config_data: Data<AntTpConfig>) -> PointerService {
    let caching_client = caching_client_data.get_ref().clone();
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let resolver_service = ResolverService::new(ant_tp_config.clone(), caching_client.clone());
    let pointer_service = PointerService::new(caching_client, ant_tp_config, resolver_service);
    pointer_service
}
