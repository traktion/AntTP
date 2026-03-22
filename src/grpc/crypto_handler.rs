use tonic::{Request, Response, Status};
use actix_web::web::Data;
use std::collections::HashMap;
use crate::service::crypto_service::{CryptoService, Crypto as ServiceVerify};

pub mod crypto_proto {
    tonic::include_proto!("crypto");
}

pub use crypto_proto::crypto_service_server::CryptoServiceServer;
use crypto_proto::crypto_service_server::CryptoService as CryptoServiceTrait;
use crypto_proto::{Crypto, CryptoRequest, CryptoResponse};

pub struct CryptoHandler {
    crypto_service: Data<CryptoService>,
}

impl CryptoHandler {
    pub fn new(crypto_service: Data<CryptoService>) -> Self {
        Self { crypto_service }
    }
}

#[tonic::async_trait]
impl CryptoServiceTrait for CryptoHandler {
    async fn verify(
        &self,
        request: Request<CryptoRequest>,
    ) -> Result<Response<CryptoResponse>, Status> {
        let req = request.into_inner();
        let public_key = req.public_key;
        
        let mut data_map = HashMap::new();
        for v in req.crypto {
            data_map.insert(v.data, ServiceVerify {
                signature: v.signature,
                verified: None,
                encrypted: None,
            });
        }

        let result_map = self.crypto_service.verify_map(public_key.clone(), data_map);

        let crypto_results = result_map.into_iter().map(|(data, v)| {
            Crypto {
                data,
                signature: v.signature,
                verified: v.verified.unwrap_or(false),
            }
        }).collect();

        Ok(Response::new(CryptoResponse {
            public_key,
            crypto: crypto_results,
        }))
    }

    async fn sign(
        &self,
        request: Request<CryptoRequest>,
    ) -> Result<Response<CryptoResponse>, Status> {
        let req = request.into_inner();
        
        let mut data_map = HashMap::new();
        for v in req.crypto {
            data_map.insert(v.data, ServiceVerify {
                signature: v.signature,
                verified: None,
                encrypted: None,
            });
        }

        let result_map = self.crypto_service.sign_map(data_map);

        let crypto_results = result_map.into_iter().map(|(data, v)| {
            Crypto {
                data,
                signature: v.signature,
                verified: v.verified.unwrap_or(false),
            }
        }).collect();

        Ok(Response::new(CryptoResponse {
            public_key: "".to_string(), // public_key not relevant for sign in response usually, or we could return app public key
            crypto: crypto_results,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use blsttc::SecretKey;

    #[tokio::test]
    async fn test_verify_grpc() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = b"test data";
        let data_hex = hex::encode(data);
        let signature = hex::encode(secret_key.sign(data).to_bytes());

        let ant_tp_config = crate::config::anttp_config::AntTpConfig::parse_from(&["anttp"]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));
        let handler = CryptoHandler::new(crypto_service);

        let request = Request::new(CryptoRequest {
            public_key: public_key.clone(),
            crypto: vec![Crypto {
                data: data_hex.clone(),
                signature: Some(signature),
                verified: false,
            }],
        });

        let response = handler.verify(request).await.unwrap();
        let inner = response.into_inner();

        assert_eq!(inner.public_key, public_key);
        assert_eq!(inner.crypto.len(), 1);
        assert!(inner.crypto[0].verified);
        assert_eq!(inner.crypto[0].data, data_hex);
    }

    #[tokio::test]
    async fn test_sign_grpc() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data_hex = hex::encode(b"hello world");

        let ant_tp_config = crate::config::anttp_config::AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));
        let handler = CryptoHandler::new(crypto_service);

        let request = Request::new(CryptoRequest {
            public_key: "".to_string(),
            crypto: vec![Crypto {
                data: data_hex.clone(),
                signature: None,
                verified: false,
            }],
        });

        let response = handler.sign(request).await.unwrap();
        let inner = response.into_inner();

        assert_eq!(inner.crypto.len(), 1);
        assert!(inner.crypto[0].verified);
        assert!(inner.crypto[0].signature.is_some());
        assert_eq!(inner.crypto[0].data, data_hex);
    }
}
