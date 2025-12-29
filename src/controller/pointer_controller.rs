use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use tokio::sync::Mutex;
use crate::client::CachingClient;
use crate::error::pointer_error::PointerError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::{cache_only, data_key};
use crate::service::access_checker::AccessChecker;
use crate::service::bookmark_resolver::BookmarkResolver;
use crate::service::pointer_name_resolver::PointerNameResolver;
use crate::service::pointer_service::{Pointer, PointerService};
use crate::service::resolver_service::ResolverService;
use crate::service::antns_resolver::AntNsResolver;

#[utoipa::path(
    post,
    path = "/anttp-0/pointer",
    request_body(
        content = Pointer
    ),
    responses(
        (status = CREATED, description = "Pointer created successfully", body = Pointer),
        (status = BAD_REQUEST, description = "Pointer body was invalid")
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
        ("x-data-key", Header, description = "Private key used to create mutable data (personal|resolver|'custom')",
        example = "personal"),
    ),
)]
pub async fn post_pointer(
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    access_checker: Data<Mutex<AccessChecker>>,
    bookmark_resolver: Data<Mutex<BookmarkResolver>>,
    pointer_name_resolver: Data<PointerNameResolver>,
    antns_resolver: Data<AntNsResolver>,
    pointer: web::Json<Pointer>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    let pointer_service = create_pointer_service(
        caching_client_data, ant_tp_config_data, access_checker, bookmark_resolver, pointer_name_resolver, antns_resolver);

    debug!("Creating new pointer");
    Ok(HttpResponse::Created().json(
        pointer_service.create_pointer(pointer.into_inner(), evm_wallet_data.get_ref().clone(), cache_only(&request), data_key(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/pointer/{address}",
    params(
        ("address", description = "Address of pointer"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
        ("x-data-key", Header, description = "Private key used to create mutable data (personal|resolver|'custom')",
        example = "personal"),
    ),
    request_body(
        content = Pointer
    ),
    responses(
        (status = OK, description = "Pointer updated successfully", body = Pointer),
        (status = BAD_REQUEST, description = "Pointer body was invalid")
    ),
)]
pub async fn put_pointer(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    access_checker: Data<Mutex<AccessChecker>>,
    bookmark_resolver: Data<Mutex<BookmarkResolver>>,
    pointer_name_resolver: Data<PointerNameResolver>,
    antns_resolver: Data<AntNsResolver>,
    pointer: web::Json<Pointer>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    let address = path.into_inner();

    let pointer_service = create_pointer_service(
        caching_client_data, ant_tp_config_data, access_checker, bookmark_resolver, pointer_name_resolver, antns_resolver);

    debug!("Updating pointer");
    Ok(HttpResponse::Ok().json(
        pointer_service.update_pointer(address, pointer.into_inner(), cache_only(&request), data_key(&request)).await?
    ))
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
    access_checker: Data<Mutex<AccessChecker>>,
    bookmark_resolver: Data<Mutex<BookmarkResolver>>,
    pointer_name_resolver: Data<PointerNameResolver>,
    antns_resolver: Data<AntNsResolver>,
) -> Result<HttpResponse, PointerError> {
    let address = path.into_inner();

    let pointer_service = create_pointer_service(
        caching_client_data, ant_tp_config_data, access_checker, bookmark_resolver, pointer_name_resolver, antns_resolver);

    debug!("Getting pointer at [{}]", address);
    Ok(HttpResponse::Ok().json(pointer_service.get_pointer(address).await?))
}

fn create_pointer_service(
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    access_checker: Data<Mutex<AccessChecker>>,
    bookmark_resolver: Data<Mutex<BookmarkResolver>>,
    pointer_name_resolver: Data<PointerNameResolver>,
    antns_resolver: Data<AntNsResolver>
) -> PointerService {
    let caching_client = caching_client_data.get_ref().clone();
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let resolver_service = ResolverService::new(
        caching_client.clone(), access_checker, bookmark_resolver, pointer_name_resolver, antns_resolver);
    PointerService::new(caching_client, ant_tp_config, resolver_service)
}
