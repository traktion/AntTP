use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::client::CachingClient;
use crate::error::register_error::RegisterError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::cache_only;
use crate::service::register_service::{Register, RegisterService};
use crate::service::resolver_service::ResolverService;

#[utoipa::path(
    post,
    path = "/anttp-0/register",
    request_body(
        content = Register
    ),
    responses(
        (status = CREATED, description = "Register created successfully", body = Register)
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_register(
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>,
    request: HttpRequest,
) -> Result<HttpResponse, RegisterError> {
    let register_service = create_register_service(caching_client_data, ant_tp_config_data);

    debug!("Creating new register");
    Ok(HttpResponse::Created().json(
        register_service.create_register(register.into_inner(), evm_wallet_data.get_ref().clone(), cache_only(request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/register/{address}",
    params(
        ("address", description = "Address of pointer"),
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory")
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
    request: HttpRequest,
) -> Result<HttpResponse, RegisterError> {
    let address = path.into_inner();

    let register_service = create_register_service(caching_client_data, ant_tp_config_data);

    debug!("Updating register");
    Ok(HttpResponse::Ok().json(
        register_service.update_register(address, register.into_inner(), evm_wallet_data.get_ref().clone(), cache_only(request)).await?
    ))
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
) -> Result<HttpResponse, RegisterError> {
    let address = path.into_inner();

    let register_service = create_register_service(caching_client_data, ant_tp_config_data);

    debug!("Getting register at [{}]", address);
    Ok(HttpResponse::Ok().json(register_service.get_register(address).await?))
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
) -> Result<HttpResponse, RegisterError> {
    let address = path.into_inner();

    let register_service = create_register_service(caching_client_data, ant_tp_config_data);

    debug!("Getting register history at [{}]", address);
    Ok(HttpResponse::Ok().json(register_service.get_register_history(address).await?))
}

fn create_register_service(caching_client_data: Data<CachingClient>, ant_tp_config_data: Data<AntTpConfig>) -> RegisterService {
    let caching_client = caching_client_data.get_ref().clone();
    let ant_tp_config = ant_tp_config_data.get_ref().clone();
    let resolver_service = ResolverService::new(ant_tp_config.clone(), caching_client.clone());
    let register_service = RegisterService::new(caching_client, ant_tp_config, resolver_service);
    register_service
}
