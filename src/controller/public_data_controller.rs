use actix_web::{web, HttpRequest, Responder};
use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Data, Payload};
use ant_evm::EvmWallet;
use log::info;
use crate::client::CachingClient;
use crate::controller::cache_only;
use crate::service::public_data_service::{PublicData, PublicDataService};

#[utoipa::path(
    post,
    path = "/anttp-0/binary/public_data",
    request_body(
        content = PublicData,
        content_type = "application/octet-stream"
    ),
    responses(
        (status = 200, description = "Public data uploaded successfully", body = PublicData),
    ),
    params(
        ("x-cache-only", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_data(
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    payload: Payload,
    request: HttpRequest
) -> impl Responder {
    let public_data_service = PublicDataService::new(caching_client_data.get_ref().clone());

    info!("Creating new public data");
    match payload.to_bytes().await {
        Ok(bytes) => {
            public_data_service.create_public_data(bytes, evm_wallet_data.get_ref().clone(), cache_only(request)).await
        }
        Err(_) => {
            Err(ErrorInternalServerError("Failed to retrieve bytes from payload"))
        }
    }
}

#[utoipa::path(
    get,
    path = "/anttp-0/binary/public_data/{address}",
    responses(
        (status = 200, description = "Public data found successfully", content_type = "application/octet-stream"),
        (status = NOT_FOUND, description = "Public data was not found")
    ),
    params(
        ("address" = String, Path, description = "Public data address"),
    )
)]
pub async fn get_public_data(
    path: web::Path<String>,
    caching_client_data: Data<CachingClient>,
) -> impl Responder {
    let address = path.into_inner();
    let public_data_service = PublicDataService::new(caching_client_data.get_ref().clone());

    info!("Getting public data at [{}]", address);
    public_data_service.get_public_data_binary(address).await
}