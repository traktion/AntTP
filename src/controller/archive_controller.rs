use actix_multipart::form::MultipartForm;
use actix_web::{web, web::Data, HttpRequest, HttpResponse};
use ant_evm::EvmWallet;
use crate::controller::get_store_type;
use crate::error::archive_error::ArchiveError;
use crate::service::archive_service::{ArchiveForm, ArchiveService, ArchiveType};

/// GET /api/v1/archive/{type}/{address}
pub async fn get_archive_root(
    path_params: web::Path<(ArchiveType, String)>,
    archive_service: Data<ArchiveService>,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address) = path_params.into_inner();
    let res = archive_service.get_archive(address, None, archive_type).await?;
    Ok(HttpResponse::Ok().json(res))
}

/// GET /api/v1/archive/{type}/{address}/{path}
pub async fn get_archive(
    path_params: web::Path<(ArchiveType, String, String)>,
    archive_service: Data<ArchiveService>,
) -> Result<HttpResponse, ArchiveError> {
    let (archive_type, address, path) = path_params.into_inner();
    let res = archive_service.get_archive(address, Some(path), archive_type).await?;
    Ok(HttpResponse::Ok().json(res))
}

/// PUT /api/v1/multipart/archive/{type}/{address}
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
