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

#[utoipa::path(
    post,
    path = "/anttp-0/crypto/sign",
    request_body(
        content = HashMap<String, Verify>,
        description = "Map of data hex to Verify struct",
        example = json!({
            "68656c6c6f20776f726c64": {
                "signature": ""
            }
        })
    ),
    responses(
        (status = OK, description = "Signing results", body = HashMap<String, Verify>),
    )
)]
pub async fn post_sign(
    crypto_service: Data<CryptoService>,
    data_map: web::Json<HashMap<String, Verify>>,
) -> HttpResponse {
    let result = crypto_service.sign(data_map.into_inner());
    HttpResponse::Ok().json(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use blsttc::SecretKey;
    use crate::service::signature_service::SignatureService;
    use crate::config::anttp_config::AntTpConfig;
    use clap::Parser;

    #[actix_web::test]
    async fn test_post_sign_success() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data_hex = hex::encode(b"hello world");

        let ant_tp_config = AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let crypto_service = Data::new(CryptoService::new(SignatureService, ant_tp_config));

        let app = test::init_service(
            App::new()
                .app_data(crypto_service.clone())
                .route("/anttp-0/crypto/sign", web::post().to(post_sign))
        ).await;

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Verify {
            signature: "".to_string(),
            verified: None,
        });

        let req = test::TestRequest::post()
            .uri("/anttp-0/crypto/sign")
            .set_json(&data_map)
            .to_request();

        let resp: HashMap<String, Verify> = test::call_and_read_body_json(&app, req).await;

        assert!(resp.contains_key(&data_hex));
        let verify_struct = resp.get(&data_hex).unwrap();
        assert!(verify_struct.verified.unwrap());
        assert!(!verify_struct.signature.is_empty());
    }

    #[actix_web::test]
    async fn test_post_verify_success() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = b"hello world";
        let data_hex = hex::encode(data);
        let signature = hex::encode(secret_key.sign(data).to_bytes());

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let crypto_service = Data::new(CryptoService::new(SignatureService, ant_tp_config));

        let app = test::init_service(
            App::new()
                .app_data(crypto_service.clone())
                .route("/anttp-0/crypto/verify/{public_key}", web::post().to(post_verify))
        ).await;

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Verify {
            signature,
            verified: None,
        });

        let req = test::TestRequest::post()
            .uri(&format!("/anttp-0/crypto/verify/{}", public_key))
            .set_json(&data_map)
            .to_request();

        let resp: HashMap<String, Verify> = test::call_and_read_body_json(&app, req).await;

        assert!(resp.contains_key(&data_hex));
        assert!(resp.get(&data_hex).unwrap().verified.unwrap());
    }
}
