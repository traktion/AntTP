use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::controller::cache_only;
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
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
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
        pnr_service.create_pnr(pnr_zone.into_inner(), evm_wallet_data.get_ref().clone(), cache_only(&request)).await?
    ))
}