use actix_http::header;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::{ContentLength, ContentType};
use actix_web::web::{Data, Payload};
use mockall_double::double;
use ant_evm::EvmWallet;
use log::debug;
use crate::controller::get_store_type;
use crate::error::public_data_error::PublicDataError;
use crate::model::key_value::KeyValue;
#[double]
use crate::service::key_value_service::KeyValueService;

#[utoipa::path(
    post,
    path = "/anttp-0/key_value/{bucket}/{object}",
    params(
        ("bucket" = String, Path, description = "Bucket name"),
        ("object" = String, Path, description = "Object name"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
    request_body(
        content = KeyValue
    ),
    responses(
        (status = CREATED, description = "Key/Value created successfully", body = KeyValue),
        (status = BAD_REQUEST, description = "Key/Value body was invalid")
    ),
)]
pub async fn post_key_value(
    key_value_service: Data<KeyValueService>,
    evm_wallet_data: Data<EvmWallet>,
    path: web::Path<(String, String)>,
    key_value: web::Json<KeyValue>,
    request: HttpRequest,
) -> Result<HttpResponse, PublicDataError> {
    let (bucket, object) = path.into_inner();
    debug!("Creating new key/value at [{}/{}]", bucket, object);
    let mut kv = key_value.into_inner();
    kv.bucket = bucket;
    kv.object = object;

    Ok(HttpResponse::Created().json(
        key_value_service.create_key_value(kv, evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
    ))
}

#[utoipa::path(
    post,
    path = "/anttp-0/binary/key_value/{bucket}/{object}",
    params(
        ("bucket" = String, Path, description = "Bucket name"),
        ("object" = String, Path, description = "Object name"),
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
    request_body(
        content = KeyValue,
        content_type = "application/octet-stream"
    ),
    responses(
        (status = CREATED, description = "Key/Value created successfully", body = KeyValue),
        (status = BAD_REQUEST, description = "Key/Value body was invalid")
    ),
)]
pub async fn post_key_value_binary(
    key_value_service: Data<KeyValueService>,
    evm_wallet_data: Data<EvmWallet>,
    path: web::Path<(String, String)>,
    payload: Payload,
    request: HttpRequest,
) -> Result<HttpResponse, PublicDataError> {
    let (bucket, object) = path.into_inner();
    debug!("Creating new binary key/value at [{}/{}]", bucket, object);

    match payload.to_bytes().await {
        Ok(bytes) => {
            key_value_service.create_key_value_binary(bucket.clone(), object.clone(), bytes, evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?;
            Ok(HttpResponse::Created().json(key_value_service.get_key_value(bucket, object).await?))
        }
        Err(e) => {
            Err(PublicDataError::GetError(crate::error::GetError::Decode(format!("Failed to retrieve bytes from payload: {}", e))))
        }
    }
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

#[utoipa::path(
    get,
    path = "/anttp-0/binary/key_value/{bucket}/{object}",
    params(
        ("bucket" = String, Path, description = "Bucket name"),
        ("object" = String, Path, description = "Object name"),
    ),
    responses(
        (status = OK, description = "Key/Value found successfully", content_type = "application/octet-stream"),
        (status = NOT_FOUND, description = "Key/Value was not found")
    )
)]
pub async fn get_key_value_binary(
    path: web::Path<(String, String)>,
    key_value_service: Data<KeyValueService>,
) -> Result<HttpResponse, PublicDataError> {
    let (bucket, object) = path.into_inner();

    debug!("Getting binary key/value at [{}/{}]", bucket, object);
    let content = key_value_service.get_key_value_binary(bucket, object).await?;
    Ok(HttpResponse::Ok()
        .insert_header(ContentType::octet_stream())
        .insert_header(ContentLength(content.len()))
        .insert_header((header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))))
        .body(content))
}
