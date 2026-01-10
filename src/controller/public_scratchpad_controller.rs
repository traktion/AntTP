use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::error::scratchpad_error::ScratchpadError;
use crate::controller::get_store_type;
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
    scratchpad_service: Data<ScratchpadService>,
    evm_wallet_data: Data<EvmWallet>,
    scratchpad: web::Json<Scratchpad>,
    request: HttpRequest,
) -> Result<HttpResponse, ScratchpadError> {
    let name = path.into_inner();

    debug!("Creating new public scratchpad");
    Ok(HttpResponse::Ok().json(
        scratchpad_service.create_scratchpad(
            name,
            scratchpad.into_inner(),
            evm_wallet_data.get_ref().clone(),
            false,
            get_store_type(&request)
        ).await?
    ))
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
    scratchpad_service: Data<ScratchpadService>,
    evm_wallet_data: Data<EvmWallet>,
    scratchpad: web::Json<Scratchpad>,
    request: HttpRequest,
) -> Result<HttpResponse, ScratchpadError> {
    let (address, name) = path.into_inner();

    debug!("Updating public scratchpad");
    Ok(HttpResponse::Ok().json(
        scratchpad_service.update_scratchpad(
            address,
            name,
            scratchpad.into_inner(),
            evm_wallet_data.get_ref().clone(),
            false,
            get_store_type(&request)
        ).await?
    ))
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
    scratchpad_service: Data<ScratchpadService>,
) -> Result<HttpResponse, ScratchpadError> {
    let address = path.into_inner();

    debug!("Getting public scratchpad at [{}]", address);
    Ok(HttpResponse::Ok().json(scratchpad_service.get_scratchpad(address, None, false).await?))
}