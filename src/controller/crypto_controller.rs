use actix_web::{web, HttpResponse};
use actix_web::web::Data;
use std::collections::HashMap;
use crate::service::crypto_service::{CryptoService, Crypto};

#[utoipa::path(
    post,
    path = "/anttp-0/crypto/verify/{public_key}",
    params(
        ("public_key" = String, Path, description = "Public key as hex string"),
    ),
    request_body(
        content = HashMap<String, Crypto>,
        description = "Map of data hex to Crypto struct",
        example = json!({
            "68656c6c6f20776f726c64": {
                "signature": "81216b208c697818836511110058e1c64e0db658092a403429399e52f0855219e273574c30c3453b6f27b9c6a7a503e9114f8263f3392f440532289659b87642630718d78f44f9449f87c53d9154497676767676767676767676767676767676"
            }
        })
    ),
    responses(
        (status = OK, description = "Verification results", body = HashMap<String, Crypto>),
    )
)]
pub async fn post_verify(
    path: web::Path<String>,
    crypto_service: Data<CryptoService>,
    data_map: web::Json<HashMap<String, Crypto>>,
) -> HttpResponse {
    let public_key = path.into_inner();
    let result = crypto_service.verify_map(public_key, data_map.into_inner());
    HttpResponse::Ok().json(result)
}

#[utoipa::path(
    post,
    path = "/anttp-0/crypto/sign",
    request_body(
        content = HashMap<String, Crypto>,
        description = "Map of data hex to Crypto struct",
        example = json!({
            "68656c6c6f20776f726c64": {
            }
        })
    ),
    responses(
        (status = OK, description = "Signing results", body = HashMap<String, Crypto>),
    )
)]
pub async fn post_sign(
    crypto_service: Data<CryptoService>,
    data_map: web::Json<HashMap<String, Crypto>>,
) -> HttpResponse {
    let result = crypto_service.sign_map(data_map.into_inner());
    HttpResponse::Ok().json(result)
}

#[utoipa::path(
    post,
    path = "/anttp-0/crypto/encrypt/{public_key}",
    params(
        ("public_key" = String, Path, description = "Public key as hex string"),
    ),
    request_body(
        content = HashMap<String, Crypto>,
        description = "Map of data to Crypto struct",
        example = json!({
            "hello world": {
            }
        })
    ),
    responses(
        (status = OK, description = "Encryption results", body = HashMap<String, Crypto>),
    )
)]
pub async fn post_encrypt(
    path: web::Path<String>,
    crypto_service: Data<CryptoService>,
    data_map: web::Json<HashMap<String, Crypto>>,
) -> HttpResponse {
    let public_key = path.into_inner();
    let result = crypto_service.encrypt_map(public_key, data_map.into_inner());
    HttpResponse::Ok().json(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use blsttc::SecretKey;
    use crate::config::anttp_config::AntTpConfig;
    use clap::Parser;

    #[actix_web::test]
    async fn test_post_sign_success() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data_hex = hex::encode(b"hello world");

        let ant_tp_config = AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));

        let app = test::init_service(
            App::new()
                .app_data(crypto_service.clone())
                .route("/anttp-0/crypto/sign", web::post().to(post_sign))
        ).await;

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Crypto {
            signature: None,
            verified: None,
            encrypted: None,
        });

        let req = test::TestRequest::post()
            .uri("/anttp-0/crypto/sign")
            .set_json(&data_map)
            .to_request();

        let resp: HashMap<String, Crypto> = test::call_and_read_body_json(&app, req).await;

        assert!(resp.contains_key(&data_hex));
        let crypto_struct = resp.get(&data_hex).unwrap();
        assert!(crypto_struct.verified.unwrap());
        assert!(crypto_struct.signature.is_some());
    }

    #[actix_web::test]
    async fn test_post_verify_success() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = b"hello world";
        let data_hex = hex::encode(data);
        let signature = hex::encode(secret_key.sign(data).to_bytes());

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));

        let app = test::init_service(
            App::new()
                .app_data(crypto_service.clone())
                .route("/anttp-0/crypto/verify/{public_key}", web::post().to(post_verify))
        ).await;

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Crypto {
            signature: Some(signature),
            verified: None,
            encrypted: None,
        });

        let req = test::TestRequest::post()
            .uri(&format!("/anttp-0/crypto/verify/{}", public_key))
            .set_json(&data_map)
            .to_request();

        let resp: HashMap<String, Crypto> = test::call_and_read_body_json(&app, req).await;

        assert!(resp.contains_key(&data_hex));
        assert!(resp.get(&data_hex).unwrap().verified.unwrap());
    }

    #[actix_web::test]
    async fn test_post_encrypt_success() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = "hello world";

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));

        let app = test::init_service(
            App::new()
                .app_data(crypto_service.clone())
                .route("/anttp-0/crypto/encrypt/{public_key}", web::post().to(post_encrypt))
        ).await;

        let mut data_map = HashMap::new();
        data_map.insert(data.to_string(), Crypto {
            signature: None,
            verified: None,
            encrypted: None,
        });

        let req = test::TestRequest::post()
            .uri(&format!("/anttp-0/crypto/encrypt/{}", public_key))
            .set_json(&data_map)
            .to_request();

        let resp: HashMap<String, Crypto> = test::call_and_read_body_json(&app, req).await;

        assert!(resp.contains_key(data));
        let encrypted_base64 = resp.get(data).unwrap().encrypted.as_ref().unwrap();

        use base64::{engine::general_purpose, Engine as _};
        use blsttc::Ciphertext;
        let encrypted_bytes = general_purpose::STANDARD.decode(encrypted_base64).unwrap();
        let ciphertext = Ciphertext::from_bytes(&encrypted_bytes).unwrap();
        let decrypted_bytes = secret_key.decrypt(&ciphertext).unwrap();
        assert_eq!(data.as_bytes(), decrypted_bytes.as_slice());
    }
}
