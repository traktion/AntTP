use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::controller::get_store_type;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::PnrZone;
use crate::service::pnr_service::PnrService;

#[utoipa::path(
    post,
    path = "/anttp-0/pnr",
    request_body(
        content = PnrZone
    ),
    responses(
        (status = CREATED, description = "PNR zone created successfully", body = PnrZone),
        (status = BAD_REQUEST, description = "PNR zone body was invalid")
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_pnr(
    pnr_service: Data<PnrService>,
    evm_wallet_data: Data<EvmWallet>,
    pnr_zone: web::Json<PnrZone>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    debug!("Creating new PNR zone");
    Ok(HttpResponse::Created().json(
        pnr_service.create_pnr(pnr_zone.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/pnr/{name}",
    params(
        ("name", description = "PNR name"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
    request_body(
        content = PnrZone
    ),
    responses(
        (status = OK, description = "PNR zone updated successfully", body = PnrZone),
        (status = BAD_REQUEST, description = "PNR zone body was invalid")
    ),
)]
pub async fn put_pnr(
    path: web::Path<String>,
    pnr_service: Data<PnrService>,
    evm_wallet_data: Data<EvmWallet>,
    pnr_zone: web::Json<PnrZone>,
    request: HttpRequest,
) -> Result<HttpResponse, PointerError> {
    let name = path.into_inner();

    debug!("Updating PNR zone");
    Ok(HttpResponse::Ok().json(
        pnr_service.update_pnr(name, pnr_zone.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    get,
    path = "/anttp-0/pnr/{name}",
    params(
        ("name", description = "PNR name"),
    ),
    responses(
        (status = OK, description = "PNR zone retrieved successfully", body = PnrZone),
        (status = NOT_FOUND, description = "PNR zone not found")
    ),
)]
pub async fn get_pnr(
    path: web::Path<String>,
    pnr_service: Data<PnrService>,
) -> Result<HttpResponse, PointerError> {
    let name = path.into_inner();

    debug!("Getting PNR zone");
    Ok(HttpResponse::Ok().json(
        pnr_service.get_pnr(name).await?
    ))
}