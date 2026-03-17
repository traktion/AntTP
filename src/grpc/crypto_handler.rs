use tonic::{Request, Response, Status};
use actix_web::web::Data;
use std::collections::HashMap;
use crate::service::crypto_service::{CryptoService, Verify as ServiceVerify};

pub mod crypto_proto {
    tonic::include_proto!("crypto");
}

pub use crypto_proto::crypto_service_server::CryptoServiceServer;
use crypto_proto::crypto_service_server::CryptoService as CryptoServiceTrait;
use crypto_proto::{Verify, VerifyRequest, VerifyResponse};

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
        request: Request<VerifyRequest>,
    ) -> Result<Response<VerifyResponse>, Status> {
        let req = request.into_inner();
        let public_key = req.public_key;
        
        let mut data_map = HashMap::new();
        for v in req.verify {
            data_map.insert(v.data, ServiceVerify {
                signature: v.signature,
                verified: None,
            });
        }

        let result_map = self.crypto_service.verify(public_key.clone(), data_map);

        let verify_results = result_map.into_iter().map(|(data, v)| {
            Verify {
                data,
                signature: v.signature,
                verified: v.verified.unwrap_or(false),
            }
        }).collect();

        Ok(Response::new(VerifyResponse {
            public_key,
            verify: verify_results,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::signature_service::SignatureService;
    use blsttc::SecretKey;

    #[tokio::test]
    async fn test_verify_grpc() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = b"test data";
        let data_hex = hex::encode(data);
        let signature = hex::encode(secret_key.sign(data).to_bytes());

        let crypto_service = Data::new(CryptoService::new(SignatureService));
        let handler = CryptoHandler::new(crypto_service);

        let request = Request::new(VerifyRequest {
            public_key: public_key.clone(),
            verify: vec![Verify {
                data: data_hex.clone(),
                signature,
                verified: false,
            }],
        });

        let response = handler.verify(request).await.unwrap();
        let inner = response.into_inner();

        assert_eq!(inner.public_key, public_key);
        assert_eq!(inner.verify.len(), 1);
        assert!(inner.verify[0].verified);
        assert_eq!(inner.verify[0].data, data_hex);
    }
}
