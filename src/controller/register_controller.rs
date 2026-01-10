use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::error::register_error::RegisterError;
use crate::controller::get_store_type;
use crate::service::register_service::{Register, RegisterService};

#[utoipa::path(
    post,
    path = "/anttp-0/register",
    request_body(
        content = Register
    ),
    responses(
        (status = CREATED, description = "Register created successfully", body = Register),
        (status = BAD_REQUEST, description = "Register body was invalid")
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_register(
    register_service: Data<RegisterService>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>,
    request: HttpRequest,
) -> Result<HttpResponse, RegisterError> {
    debug!("Creating new register");
    Ok(HttpResponse::Created().json(
        register_service.create_register(
            register.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
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
        (status = OK, description = "Register updated successfully", body = Register),
        (status = BAD_REQUEST, description = "Register body was invalid")
    ),
)]
pub async fn put_register(
    path: web::Path<String>,
    register_service: Data<RegisterService>,
    evm_wallet_data: Data<EvmWallet>,
    register: web::Json<Register>,
    request: HttpRequest,
) -> Result<HttpResponse, RegisterError> {
    let address = path.into_inner();
    debug!("Updating register");
    Ok(HttpResponse::Ok().json(
        register_service.update_register(
            address, register.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
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
    path: web::Path<String>,
    register_service: Data<RegisterService>,
) -> Result<HttpResponse, RegisterError> {
    let address = path.into_inner();
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
    path: web::Path<String>,
    register_service: Data<RegisterService>,
) -> Result<HttpResponse, RegisterError> {
    let address = path.into_inner();
    debug!("Getting register history at [{}]", address);
    Ok(HttpResponse::Ok().json(register_service.get_register_history(address).await?))
}
