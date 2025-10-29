use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::config::anttp_config::AntTpConfig;
use crate::{UploaderState, UploadState};
use crate::service::public_archive_service::{PublicArchiveForm, PublicArchiveService, Upload};
use crate::client::CachingClient;
use crate::error::public_archive_error::PublicArchiveError;
use crate::controller::cache_only;
use crate::service::file_service::FileService;

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
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_archive(
    public_archive_form: MultipartForm<PublicArchiveForm>,
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    ant_tp_config: Data<AntTpConfig>,
    request: HttpRequest
) -> Result<HttpResponse, PublicArchiveError> {
    let archive_service = build_archive_service(
        caching_client_data,
        uploader_state,
        upload_state,
        ant_tp_config.clone()
    );
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Creating new archive from multipart POST");
    Ok(HttpResponse::Created().json(
        archive_service.create_public_archive(public_archive_form, evm_wallet, cache_only(request)).await?
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
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn put_public_archive(
    path: web::Path<String>,
    public_archive_form: MultipartForm<PublicArchiveForm>,
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    ant_tp_config: Data<AntTpConfig>,
    request: HttpRequest,
) -> Result<HttpResponse, PublicArchiveError> {
    let address = path.into_inner();
    let archive_service = build_archive_service(
        caching_client_data,
        uploader_state,
        upload_state,
        ant_tp_config.clone()
    );
    let evm_wallet = evm_wallet_data.get_ref().clone();

    debug!("Updating [{}] archive from multipart PUT with cache_only [{:?}]", address, cache_only(request.clone()));
    Ok(HttpResponse::Ok().json(
        archive_service.update_public_archive(address, public_archive_form, evm_wallet, cache_only(request)).await?
    ))
}

#[utoipa::path(
    get,
    path = "/anttp-0/public_archive/status/{id}",
    responses(
        (status = OK, description = "Id found successfully", body = Upload),
        (status = NOT_FOUND, description = "Id was not found")
    ),
    params(
        ("id" = String, Path, description = "Id of upload"),
    )
)]
pub async fn get_status_public_archive(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    ant_tp_config: Data<AntTpConfig>,
) -> Result<HttpResponse, PublicArchiveError> {
    let id = path.into_inner();
    let archive_service = build_archive_service(
        caching_client_data,
        uploader_state,
        upload_state,
        ant_tp_config.clone()
    );

    debug!("Checking upload status for [{}]", id);
    Ok(HttpResponse::Ok().json(archive_service.get_status(id).await?))
}

fn build_archive_service(
    caching_client_data: Data<CachingClient>,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> PublicArchiveService {
    let ant_tp_config = ant_tp_config_data.get_ref();
    let caching_client = caching_client_data.get_ref();
    let file_service = FileService::new(caching_client.clone(), ant_tp_config.clone());
    PublicArchiveService::new(file_service, uploader_state, upload_state, caching_client.clone())
}
