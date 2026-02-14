use actix_multipart::form::MultipartForm;
use actix_web::{web, web::Data, HttpRequest, HttpResponse};
use ant_evm::EvmWallet;
use crate::controller::get_store_type;
use crate::error::archive_error::ArchiveError;
use crate::service::archive_service::{ArchiveForm, ArchiveResponse, ArchiveService, ArchiveType, Upload};

/// GET /api/v1/archive/{type}/{address}
#[utoipa::path(
    get,
    path = "/api/v1/archive/{type}/{address}",
    responses(
        (status = OK, description = "Archive retrieved successfully", body = ArchiveResponse),
        (status = NOT_FOUND, description = "Archive not found")
    ),
    params(
        ("type" = ArchiveType, Path, description = "Archive type: public or tarchive"),
        ("address" = String, Path, description = "Archive address")
    )
)]
pub async fn get_archive_root(
    path_params: web::Path<(ArchiveType, String)>,
    archive_service: Data<ArchiveService>,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address) = path_params.into_inner();
    let res = archive_service.get_archive(address, None, archive_type).await?;
    Ok(HttpResponse::Ok().json(res))
}

/// GET /api/v1/archive/{type}/{address}/{path}
#[utoipa::path(
    get,
    path = "/api/v1/archive/{type}/{address}/{path}",
    responses(
        (status = OK, description = "Archive content retrieved successfully", body = ArchiveResponse),
        (status = NOT_FOUND, description = "Archive or path not found")
    ),
    params(
        ("type" = ArchiveType, Path, description = "Archive type: public or tarchive"),
        ("address" = String, Path, description = "Archive address"),
        ("path" = String, Path, description = "Path within the archive")
    )
)]
pub async fn get_archive(
    path_params: web::Path<(ArchiveType, String, String)>,
    archive_service: Data<ArchiveService>,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address, path) = path_params.into_inner();
    let res = archive_service.get_archive(address, Some(path), archive_type).await?;
    Ok(HttpResponse::Ok().json(res))
}

/// PUT /api/v1/multipart/archive/{type}/{address}
#[utoipa::path(
    put,
    path = "/api/v1/multipart/archive/{type}/{address}",
    request_body(content = ArchiveForm, content_type = "multipart/form-data"),
    responses(
        (status = OK, description = "Archive updated successfully", body = ArchiveResponse)
    ),
    params(
        ("type" = ArchiveType, Path, description = "Archive type: public or tarchive"),
        ("address" = String, Path, description = "Archive address"),
        ("x-store-type" = Option<String>, Header, description = "Store type: memory, disk or network")
    )
)]
pub async fn put_archive_root(
    path_params: web::Path<(ArchiveType, String)>,
    archive_form: MultipartForm<ArchiveForm>,
    archive_service: Data<ArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address) = path_params.into_inner();
    let store_type = get_store_type(&request);
    let res = archive_service
        .update_archive(
            address,
            None,
            archive_form,
            evm_wallet_data.get_ref().clone(),
            store_type,
            archive_type,
        )
        .await?;
    Ok(HttpResponse::Ok().json(res))
}

/// PUT /api/v1/multipart/archive/{type}/{address}/{path}
#[utoipa::path(
    put,
    path = "/api/v1/multipart/archive/{type}/{address}/{path}",
    request_body(content = ArchiveForm, content_type = "multipart/form-data"),
    responses(
        (status = OK, description = "Archive updated successfully", body = ArchiveResponse)
    ),
    params(
        ("type" = ArchiveType, Path, description = "Archive type: public or tarchive"),
        ("address" = String, Path, description = "Archive address"),
        ("path" = String, Path, description = "Target path within the archive"),
        ("x-store-type" = Option<String>, Header, description = "Store type: memory, disk or network")
    )
)]
pub async fn put_archive(
    path_params: web::Path<(ArchiveType, String, String)>,
    archive_form: MultipartForm<ArchiveForm>,
    archive_service: Data<ArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address, path) = path_params.into_inner();
    let store_type = get_store_type(&request);
    let res = archive_service
        .update_archive(
            address,
            Some(path),
            archive_form,
            evm_wallet_data.get_ref().clone(),
            store_type,
            archive_type,
        )
        .await?;
    Ok(HttpResponse::Ok().json(res))
}

/// DELETE /api/v1/archive/{type}/{address}/{path}
#[utoipa::path(
    delete,
    path = "/api/v1/archive/{type}/{address}/{path}",
    responses(
        (status = OK, description = "Archive truncated successfully", body = Upload)
    ),
    params(
        ("type" = ArchiveType, Path, description = "Archive type: public or tarchive"),
        ("address" = String, Path, description = "Archive address"),
        ("path" = String, Path, description = "Path to truncate"),
        ("x-store-type" = Option<String>, Header, description = "Store type: memory, disk or network")
    )
)]
pub async fn delete_archive(
    path_params: web::Path<(ArchiveType, String, String)>,
    archive_service: Data<ArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address, path) = path_params.into_inner();
    let store_type = get_store_type(&request);
    let res = archive_service
        .truncate_archive(
            address,
            path,
            evm_wallet_data.get_ref().clone(),
            store_type,
            archive_type,
        )
        .await?;
    Ok(HttpResponse::Ok().json(res))
}

/// POST /api/v1/archive/{type}/{address} (push)
#[utoipa::path(
    post,
    path = "/api/v1/archive/{type}/{address}",
    responses(
        (status = OK, description = "Archive pushed successfully", body = Upload)
    ),
    params(
        ("type" = ArchiveType, Path, description = "Archive type: public or tarchive"),
        ("address" = String, Path, description = "Archive address"),
        ("x-store-type" = Option<String>, Header, description = "Store type: memory, disk or network")
    )
)]
pub async fn push_archive(
    path_params: web::Path<(ArchiveType, String)>,
    archive_service: Data<ArchiveService>,
    evm_wallet_data: Data<EvmWallet>,
    request: HttpRequest,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address) = path_params.into_inner();
    let store_type = get_store_type(&request);
    let res = archive_service
        .push_archive(
            address,
            evm_wallet_data.get_ref().clone(),
            store_type,
            archive_type,
        )
        .await?;
    Ok(HttpResponse::Ok().json(res))
}
