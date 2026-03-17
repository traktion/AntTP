use actix_web::{web, HttpResponse};
use actix_web::web::Data;
use std::collections::HashMap;
use crate::service::crypto_service::{CryptoService, Verify};

#[utoipa::path(
    post,
    path = "/anttp-0/crypto/verify/{public_key}",
    params(
        ("public_key" = String, Path, description = "Public key as hex string"),
    ),
    request_body(
        content = HashMap<String, Verify>,
        description = "Map of data hex to Verify struct",
        example = json!({
            "68656c6c6f20776f726c64": {
                "signature": "81216b208c697818836511110058e1c64e0db658092a403429399e52f0855219e273574c30c3453b6f27b9c6a7a503e9114f8263f3392f440532289659b87642630718d78f44f9449f87c53d9154497676767676767676767676767676767676"
            }
        })
    ),
    responses(
        (status = OK, description = "Verification results", body = HashMap<String, Verify>),
    )
)]
pub async fn post_verify(
    path: web::Path<String>,
    crypto_service: Data<CryptoService>,
    data_map: web::Json<HashMap<String, Verify>>,
) -> HttpResponse {
    let public_key = path.into_inner();
    let result = crypto_service.verify(public_key, data_map.into_inner());
    HttpResponse::Ok().json(result)
}
