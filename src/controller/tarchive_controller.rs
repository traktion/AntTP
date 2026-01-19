use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::service::tarchive_service::{TarchiveForm, TarchiveService, TarchiveUpload};
use crate::error::tarchive_error::TarchiveError;
use crate::controller::get_store_type;

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/tarchive",
    request_body(
        content = TarchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = CREATED, description = "Tarchive created successfully", body = TarchiveUpload)
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_tarchive(
    tarchive_form: MultipartForm<TarchiveForm>,
    tarchive_service: Data<TarchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest
) -> Result<HttpResponse, TarchiveError> {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new tarchive from multipart POST");
    Ok(HttpResponse::Created().json(
        tarchive_service.create_tarchive(tarchive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/multipart/tarchive/{address}",
    request_body(
        content = TarchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Tarchive updated successfully", body = TarchiveUpload)
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_tarchive(
    path: web::Path<String>,
    tarchive_form: MultipartForm<TarchiveForm>,
    tarchive_service: Data<TarchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, TarchiveError> {
    let address = path.into_inner();
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Updating [{}] tarchive from multipart PUT with store type [{:?}]", address, get_store_type(&request));
    Ok(HttpResponse::Ok().json(
        tarchive_service.update_tarchive(address, tarchive_form, evm_wallet, get_store_type(&request)).await?
    ))
}
