use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::error::scratchpad_error::ScratchpadError;
use crate::controller::get_store_type;
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
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_private_scratchpad(
    scratchpad_service: Data<ScratchpadService>,
    evm_wallet_data: Data<EvmWallet>,
    scratchpad: web::Json<Scratchpad>,
    request: HttpRequest,
) -> Result<HttpResponse, ScratchpadError> {
    debug!("Creating new private scratchpad");
    Ok(HttpResponse::Ok().json(
        scratchpad_service.create_scratchpad(
            scratchpad.into_inner(),
            evm_wallet_data.get_ref().clone(),
            true,
            get_store_type(&request)
        ).await?
    ))
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
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_private_scratchpad(
    path: web::Path<(String, String)>,
    scratchpad_service: Data<ScratchpadService>,
    evm_wallet_data: Data<EvmWallet>,
    scratchpad: web::Json<Scratchpad>,
    request: HttpRequest,
) -> Result<HttpResponse, ScratchpadError> {
    let (address, name) = path.into_inner();

    debug!("Updating private scratchpad");
    Ok(HttpResponse::Ok().json(
        scratchpad_service.update_scratchpad(
            address,
            name,
            scratchpad.into_inner(),
            evm_wallet_data.get_ref().clone(),
            true,
            get_store_type(&request)
        ).await?
    ))
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
    scratchpad_service: Data<ScratchpadService>,
) -> Result<HttpResponse, ScratchpadError> {
    let (address, name) = path.into_inner();

    debug!("Getting private scratchpad at [{}] with name [{}]", address, name);
    Ok(HttpResponse::Ok().json(scratchpad_service.get_scratchpad(address, Some(name), true).await?))
}