use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::controller::get_store_type;
use crate::error::public_data_error::PublicDataError;
use crate::model::key_value::KeyValue;
use crate::service::key_value_service::KeyValueService;

#[utoipa::path(
    post,
    path = "/anttp-0/key_value",
    request_body(
        content = KeyValue
    ),
    responses(
        (status = CREATED, description = "Key/Value created successfully", body = KeyValue),
        (status = BAD_REQUEST, description = "Key/Value body was invalid")
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_key_value(
    key_value_service: Data<KeyValueService>,
    evm_wallet_data: Data<EvmWallet>,
    key_value: web::Json<KeyValue>,
    request: HttpRequest,
) -> Result<HttpResponse, PublicDataError> {
    debug!("Creating new key/value");
    Ok(HttpResponse::Created().json(
        key_value_service.create_key_value(key_value.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    get,
    path = "/anttp-0/key_value/{bucket}/{object}",
    params(
        ("bucket" = String, Path, description = "Bucket name"),
        ("object" = String, Path, description = "Object name"),
    ),
    responses(
        (status = OK, description = "Key/Value found successfully", body = KeyValue),
        (status = NOT_FOUND, description = "Key/Value was not found")
    )
)]
pub async fn get_key_value(
    path: web::Path<(String, String)>,
    key_value_service: Data<KeyValueService>,
) -> Result<HttpResponse, PublicDataError> {
    let (bucket, object) = path.into_inner();

    debug!("Getting key/value at [{}/{}]", bucket, object);
    Ok(HttpResponse::Ok().json(key_value_service.get_key_value(bucket, object).await?))
}
