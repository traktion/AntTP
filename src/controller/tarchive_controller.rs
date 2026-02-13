use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::service::public_archive_service::{PublicArchiveForm, Upload};
use crate::service::tarchive_service::TarchiveService;
use crate::service::public_data_service::PublicDataService;
use crate::error::tarchive_error::TarchiveError;
use crate::controller::get_store_type;

#[utoipa::path(
    post,
    path = "/anttp-0/tarchive/{address}",
    responses(
        (status = OK, description = "Tarchive pushed successfully", body = Upload)
    ),
    params(
        ("address" = String, Path, description = "Tarchive address"),
        ("x-store-type", Header, description = "Target store type (memory|disk|network)", example = "network"),
    ),
)]
pub async fn push_tarchive(
    path: web::Path<String>,
    public_data_service: Data<PublicDataService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, TarchiveError> {
    let address = path.into_inner();
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Pushing tarchive [{}] to target store type [{:?}]", address, get_store_type(&request));
    Ok(HttpResponse::Ok().json(
        public_data_service.push_public_data(address, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/tarchive",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = CREATED, description = "Tarchive created successfully", body = Upload)
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_tarchive_root(
    tarchive_form: MultipartForm<PublicArchiveForm>,
    tarchive_service: Data<TarchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest
) -> Result<HttpResponse, TarchiveError> {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new tarchive from multipart POST");
    Ok(HttpResponse::Created().json(
        tarchive_service.create_tarchive(None, tarchive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/tarchive/{path}",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = CREATED, description = "Tarchive created successfully", body = Upload)
    ),
    params(
        ("path" = String, Path, description = "Target path (directory) for all uploads"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_tarchive(
    path_params: web::Path<String>,
    tarchive_form: MultipartForm<PublicArchiveForm>,
    tarchive_service: Data<TarchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest
) -> Result<HttpResponse, TarchiveError> {
    let mut path = path_params.into_inner();
    path = path.replace("%2F", "/");
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new tarchive from multipart POST at path [{}]", path);
    Ok(HttpResponse::Created().json(
        tarchive_service.create_tarchive(Some(path), tarchive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/multipart/tarchive/{address}",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Tarchive updated successfully", body = Upload)
    ),
    params(
        ("address" = String, Path, description = "Tarchive address"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_tarchive_root(
    path: web::Path<String>,
    tarchive_form: MultipartForm<PublicArchiveForm>,
    tarchive_service: Data<TarchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, TarchiveError> {
    let address = path.into_inner();
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Updating [{}] tarchive from multipart PUT with store type [{:?}]", address, get_store_type(&request));
    Ok(HttpResponse::Ok().json(
        tarchive_service.update_tarchive(address, None, tarchive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/multipart/tarchive/{address}/{path}",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Tarchive updated successfully", body = Upload)
    ),
    params(
        ("address" = String, Path, description = "Tarchive address"),
        ("path" = String, Path, description = "Target path (directory) for all uploads"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_tarchive(
    path_params: web::Path<(String, String)>,
    tarchive_form: MultipartForm<PublicArchiveForm>,
    tarchive_service: Data<TarchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, TarchiveError> {
    let (address, mut path) = path_params.into_inner();
    path = path.replace("%2F", "/");
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Updating [{}] tarchive from multipart PUT at path [{}] with store type [{:?}]", address, path, get_store_type(&request));
    Ok(HttpResponse::Ok().json(
        tarchive_service.update_tarchive(address, Some(path), tarchive_form, evm_wallet, get_store_type(&request)).await?
    ))
}
