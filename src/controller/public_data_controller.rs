use actix_http::header;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::{ContentLength, ContentType};
use actix_web::web::{Data, Payload};
use ant_evm::EvmWallet;
use log::debug;
use crate::error::public_data_error::PublicDataError;
use crate::controller::get_store_type;
use crate::error::CreateError;
use crate::service::public_data_service::{PublicData, PublicDataService};

#[utoipa::path(
    post,
    path = "/anttp-0/binary/public_data",
    request_body(
        content = PublicData,
        content_type = "application/octet-stream"
    ),
    responses(
        (status = CREATED, description = "Public data uploaded successfully", body = PublicData),
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_public_data(
    public_data_service: Data<PublicDataService>,
    evm_wallet_data: Data<EvmWallet>,
    payload: Payload,
    request: HttpRequest
) -> Result<HttpResponse, PublicDataError> {
    debug!("Creating new public data");
    match payload.to_bytes().await {
        Ok(bytes) => {
            Ok(HttpResponse::Created().json(
                public_data_service.create_public_data(bytes, evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
            ))
        }
        Err(e) => {
            Err(CreateError::InvalidData(e.to_string()).into())
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
    public_data_service: Data<PublicDataService>,
) -> Result<HttpResponse, PublicDataError> {
    let address = path.into_inner();

    debug!("Getting public data at [{}]", address);
    let bytes = public_data_service.get_public_data_binary(address).await?;
    // todo: add caching headers (etag, etc)
    Ok(HttpResponse::Ok()
        .insert_header(ContentType::octet_stream())
        .insert_header(ContentLength(bytes.len()))
        .insert_header((header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))))
        .body(bytes))
}
