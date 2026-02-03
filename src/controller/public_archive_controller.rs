use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::service::public_archive_service::{ArchiveContent, PublicArchiveForm, PublicArchiveService, Upload};
use crate::error::public_archive_error::PublicArchiveError;
use crate::controller::get_store_type;

#[utoipa::path(
    get,
    path = "/anttp-0/public_archive/{address}/{path}",
    responses(
        (status = OK, description = "Public archive retrieved successfully", body = ArchiveContent)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
        ("path" = Option<String>, Path, description = "Path within the archive"),
    ),
)]
pub async fn get_public_archive(
    path_params: web::Path<(String, Option<String>)>,
    public_archive_service: Data<PublicArchiveService>,
) -> Result<HttpResponse, PublicArchiveError> {
    let (address, path) = path_params.into_inner();
    debug!("Retrieving public archive at [{}] with path [{:?}]", address, path);
    Ok(HttpResponse::Ok().json(
        public_archive_service.get_public_archive(address, path).await?
    ))
}

#[utoipa::path(
    post,
    path = "/anttp-0/multipart/public_archive",
    request_body(
        content = PublicArchiveForm,
        content_type = "multipart/form-data"
    ),
    responses(
        (status = CREATED, description = "Public archive created successfully", body = Upload)
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_archive(
    public_archive_form: MultipartForm<PublicArchiveForm>,
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest
) -> Result<HttpResponse, PublicArchiveError> {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new archive from multipart POST");
    Ok(HttpResponse::Created().json(
        public_archive_service.create_public_archive(public_archive_form, evm_wallet, get_store_type(&request)).await?
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
        (status = OK, description = "Public archive updated successfully", body = Upload)
    ),
    params(
        ("address" = String, Path, description = "Public archive address"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_public_archive(
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
        public_archive_service.update_public_archive(address, public_archive_form, evm_wallet, get_store_type(&request)).await?
    ))
}
