use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::service::signature_service::SignatureService;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct Verify {
    pub signature: String,
    pub verified: Option<bool>,
}

#[derive(Debug)]
pub struct CryptoService {
    signature_service: SignatureService,
}

impl CryptoService {
    pub fn new(signature_service: SignatureService) -> Self {
        Self { signature_service }
    }

    pub fn verify(&self, public_key: String, mut data_map: HashMap<String, Verify>) -> HashMap<String, Verify> {
        for (data_hex, verify_struct) in data_map.iter_mut() {
            let is_verified = self.signature_service.verify_hex(&public_key, &verify_struct.signature, data_hex);
            verify_struct.verified = Some(is_verified);
        }
        data_map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blsttc::SecretKey;

    #[test]
    fn test_verify_success() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = b"test data";
        let data_hex = hex::encode(data);
        let signature = hex::encode(secret_key.sign(data).to_bytes());

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Verify {
            signature,
            verified: None,
        });

        let service = CryptoService::new(SignatureService);
        let result = service.verify(public_key, data_map);

        assert!(result.get(&data_hex).unwrap().verified.unwrap());
    }

    #[test]
    fn test_verify_failure() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data_hex = hex::encode(b"test data");
        let signature = hex::encode(secret_key.sign(b"other data").to_bytes());

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Verify {
            signature,
            verified: None,
        });

        let service = CryptoService::new(SignatureService);
        let result = service.verify(public_key, data_map);

        assert!(!result.get(&data_hex).unwrap().verified.unwrap());
    }
}
