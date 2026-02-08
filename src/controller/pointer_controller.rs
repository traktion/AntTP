use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::error::pointer_error::PointerError;
use crate::controller::{get_store_type, data_key};
use crate::service::pointer_service::{Pointer, PointerService};

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
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
        ("x-data-key", Header, description = "Private key used to create mutable data (personal|resolver|'custom')",
        example = "personal"),
    ),
)]
pub async fn post_pointer(
    pointer_service: Data<PointerService>,
    evm_wallet_data: Data<EvmWallet>,
    pointer: web::Json<Pointer>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    debug!("Creating new pointer");
    Ok(HttpResponse::Created().json(
        pointer_service.create_pointer(pointer.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request), data_key(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/pointer/{address}",
    params(
        ("address", description = "Address of pointer"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
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
    pointer_service: Data<PointerService>,
    pointer: web::Json<Pointer>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    let address = path.into_inner();

    debug!("Updating pointer");
    Ok(HttpResponse::Ok().json(
        pointer_service.update_pointer(address, pointer.into_inner(), get_store_type(&request), data_key(&request)).await?
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
    pointer_service: Data<PointerService>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    let address = path.into_inner();

    debug!("Getting pointer at [{}]", address);
    Ok(HttpResponse::Ok().json(pointer_service.get_pointer(address, data_key(&request)).await?))
}
