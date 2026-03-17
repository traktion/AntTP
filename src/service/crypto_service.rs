use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::config::anttp_config::AntTpConfig;
use crate::service::signature_service::SignatureService;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct Verify {
    pub signature: String,
    pub verified: Option<bool>,
}

#[derive(Debug)]
pub struct CryptoService {
    signature_service: SignatureService,
    ant_tp_config: AntTpConfig,
}

impl CryptoService {
    pub fn new(signature_service: SignatureService, ant_tp_config: AntTpConfig) -> Self {
        Self { signature_service, ant_tp_config }
    }

    pub fn sign(&self, mut data_map: HashMap<String, Verify>) -> HashMap<String, Verify> {
        match self.ant_tp_config.get_app_private_key() {
            Ok(app_private_key) => {
                for (data_hex, verify_struct) in data_map.iter_mut() {
                    match hex::decode(data_hex) {
                        Ok(data_bytes) => {
                            let signature = app_private_key.sign(&data_bytes);
                            verify_struct.signature = hex::encode(signature.to_bytes());
                            verify_struct.verified = Some(true);
                        }
                        Err(_) => {
                            verify_struct.verified = Some(false);
                        }
                    }
                }
            }
            Err(_) => {
                for verify_struct in data_map.values_mut() {
                    verify_struct.verified = Some(false);
                }
            }
        }
        data_map
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
    use clap::Parser;

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

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let service = CryptoService::new(SignatureService, ant_tp_config);
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

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let service = CryptoService::new(SignatureService, ant_tp_config);
        let result = service.verify(public_key, data_map);

        assert!(!result.get(&data_hex).unwrap().verified.unwrap());
    }

    #[test]
    fn test_sign_success() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data = b"test data";
        let data_hex = hex::encode(data);

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Verify {
            signature: "".to_string(),
            verified: None,
        });

        let ant_tp_config = AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let service = CryptoService::new(SignatureService, ant_tp_config);
        let result = service.sign(data_map);

        let verify_struct = result.get(&data_hex).unwrap();
        assert!(verify_struct.verified.unwrap());
        assert!(!verify_struct.signature.is_empty());

        // Verify the generated signature
        let is_verified = SignatureService.verify_hex(
            &hex::encode(secret_key.public_key().to_bytes()),
            &verify_struct.signature,
            &data_hex
        );
        assert!(is_verified);
    }
}
