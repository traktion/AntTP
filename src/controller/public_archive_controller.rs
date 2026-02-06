use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::service::public_archive_service::{PublicArchiveForm, PublicArchiveService, Upload, PublicArchiveResponse};
use crate::error::public_archive_error::PublicArchiveError;
use crate::controller::get_store_type;

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/public_archive",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = CREATED, description = "Public archive created successfully", body = PublicArchiveResponse)
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_archive_root(
    public_archive_form: MultipartForm<PublicArchiveForm>,
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest
) -> Result<HttpResponse, PublicArchiveError> {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new archive from multipart POST");
    Ok(HttpResponse::Created().json(
        public_archive_service.create_public_archive(None, public_archive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/public_archive/{path}",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = CREATED, description = "Public archive created successfully", body = PublicArchiveResponse)
    ),
    params(
        ("path" = String, Path, description = "Target path (directory) for all uploads"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_archive(
    path_params: web::Path<String>,
    public_archive_form: MultipartForm<PublicArchiveForm>,
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest
) -> Result<HttpResponse, PublicArchiveError> {
    let mut path = path_params.into_inner();
    path = path.replace("%2F", "/");
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new archive from multipart POST at path [{}]", path);
    Ok(HttpResponse::Created().json(
        public_archive_service.create_public_archive(Some(path), public_archive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/multipart/public_archive/{address}",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Public archive updated successfully", body = PublicArchiveResponse)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_public_archive_root(
    path: web::Path<String>,
    public_archive_form: MultipartForm<PublicArchiveForm>,
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, PublicArchiveError> {
    let address = path.into_inner();
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Updating [{}] archive from multipart PUT with store type [{:?}]", address, get_store_type(&request));
    Ok(HttpResponse::Ok().json(
        public_archive_service.update_public_archive(address, None, public_archive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    put,
    path = "/anttp-0/multipart/public_archive/{address}/{path}",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = OK, description = "Public archive updated successfully", body = PublicArchiveResponse)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
        ("path" = String, Path, description = "Target path (directory) for all uploads"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_public_archive(
    path_params: web::Path<(String, String)>,
    public_archive_form: MultipartForm<PublicArchiveForm>,
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, PublicArchiveError> {
    let (address, mut path) = path_params.into_inner();
    path = path.replace("%2F", "/");
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Updating [{}] archive from multipart PUT at path [{}] with store type [{:?}]", address, path, get_store_type(&request));
    Ok(HttpResponse::Ok().json(
        public_archive_service.update_public_archive(address, Some(path), public_archive_form, evm_wallet, get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    get,
    path = "/anttp-0/public_archive/{address}",
    responses(
        (status = OK, description = "Public archive retrieved successfully", body = PublicArchiveResponse)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
    ),
)]
pub async fn get_public_archive_root(
    path_params: web::Path<String>,
    public_archive_service: Data<PublicArchiveService>,
) -> Result<HttpResponse, PublicArchiveError> {
    let address = path_params.into_inner();
    debug!("Getting public archive root for address [{}]", address);
    Ok(HttpResponse::Ok().json(
        public_archive_service.get_public_archive(address, None).await?
    ))
}

#[utoipa::path(
    get,
    path = "/anttp-0/public_archive/{address}/{path}",
    responses(
        (status = OK, description = "Public archive retrieved successfully", body = PublicArchiveResponse)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
        ("path" = String, Path, description = "Path to directory or file within the archive"),
    ),
)]
pub async fn get_public_archive(
    path_params: web::Path<(String, String)>,
    public_archive_service: Data<PublicArchiveService>,
) -> Result<HttpResponse, PublicArchiveError> {
    let (address, mut path) = path_params.into_inner();
    path = path.replace("%2F", "/");
    debug!("Getting public archive for address [{}] and path [{}]", address, path);
    Ok(HttpResponse::Ok().json(
        public_archive_service.get_public_archive(address, Some(path)).await?
    ))
}

#[utoipa::path(
    delete,
    path = "/anttp-0/public_archive/{address}/{path}",
    responses(
        (status = OK, description = "Public archive truncated successfully", body = Upload)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
        ("path" = String, Path, description = "Path to directory or file within the archive to be deleted"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn delete_public_archive(
    path_params: web::Path<(String, String)>,
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, PublicArchiveError> {
    let (address, mut path) = path_params.into_inner();
    path = path.replace("%2F", "/");
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Truncating public archive at address [{}] and path [{}]", address, path);
    Ok(HttpResponse::Ok().json(
        public_archive_service.truncate_public_archive(address, path, evm_wallet, get_store_type(&request)).await?
    ))
}
