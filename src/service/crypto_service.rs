use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use blsttc::{PublicKey, Signature};
use crate::config::anttp_config::AntTpConfig;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct Crypto {
    pub signature: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CryptoService {
    ant_tp_config: AntTpConfig,
}

impl CryptoService {
    pub fn new(ant_tp_config: AntTpConfig) -> Self {
        Self { ant_tp_config }
    }

    pub fn sign_map(&self, mut data_map: HashMap<String, Crypto>) -> HashMap<String, Crypto> {
        for (data_hex, crypto_struct) in data_map.iter_mut() {
            match self.sign(data_hex) {
                Some(signature_hex) => {
                    crypto_struct.signature = Some(signature_hex);
                    crypto_struct.verified = Some(true);
                }
                None => {
                    crypto_struct.verified = Some(false);
                }
            }
        }
        data_map
    }

    pub fn sign(&self, data_hex: &str) -> Option<String> {
        match self.ant_tp_config.get_app_private_key() {
            Ok(app_private_key) => {
                match hex::decode(data_hex) {
                    Ok(data_bytes) => {
                        let signature = app_private_key.sign(&data_bytes);
                        Some(hex::encode(signature.to_bytes()))
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    pub fn verify_map(&self, public_key: String, mut data_map: HashMap<String, Crypto>) -> HashMap<String, Crypto> {
        for (data_hex, crypto_struct) in data_map.iter_mut() {
            let signature = crypto_struct.signature.clone().unwrap_or_default();
            let is_verified = self.verify(&public_key, &signature, data_hex);
            crypto_struct.verified = Some(is_verified);
        }
        data_map
    }

    pub fn verify(&self, public_key_hex: &str, signature_hex: &str, data_hex: &str) -> bool {
        let public_key_bytes = match hex::decode(public_key_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        let signature_bytes = match hex::decode(signature_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        let data_bytes = match hex::decode(data_hex) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let mut pk_arr = [0u8; 48];
        if public_key_bytes.len() != 48 {
            return false;
        }
        pk_arr.copy_from_slice(&public_key_bytes);
        let public_key = match PublicKey::from_bytes(pk_arr) {
            Ok(pk) => pk,
            Err(_) => return false,
        };

        let mut sig_arr = [0u8; 96];
        if signature_bytes.len() != 96 {
            return false;
        }
        sig_arr.copy_from_slice(&signature_bytes);
        let signature = match Signature::from_bytes(sig_arr) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        public_key.verify(&signature, &data_bytes)
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
        data_map.insert(data_hex.clone(), Crypto {
            signature: Some(signature),
            verified: None,
        });

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let service = CryptoService::new(ant_tp_config);
        let result = service.verify_map(public_key, data_map);

        assert!(result.get(&data_hex).unwrap().verified.unwrap());
    }

    #[test]
    fn test_verify_failure() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data_hex = hex::encode(b"test data");
        let signature = hex::encode(secret_key.sign(b"other data").to_bytes());

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Crypto {
            signature: Some(signature),
            verified: None,
        });

        let ant_tp_config = AntTpConfig::parse_from(&["anttp"]);
        let service = CryptoService::new(ant_tp_config);
        let result = service.verify_map(public_key, data_map);

        assert!(!result.get(&data_hex).unwrap().verified.unwrap());
    }

    #[test]
    fn test_sign_individual_success() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data = b"test data";
        let data_hex = hex::encode(data);

        let ant_tp_config = AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let service = CryptoService::new(ant_tp_config);
        let signature = service.sign(&data_hex);

        assert!(signature.is_some());
        let is_verified = service.verify(
            &hex::encode(secret_key.public_key().to_bytes()),
            &signature.unwrap(),
            &data_hex
        );
        assert!(is_verified);
    }

    #[test]
    fn test_sign_success() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data = b"test data";
        let data_hex = hex::encode(data);

        let mut data_map = HashMap::new();
        data_map.insert(data_hex.clone(), Crypto {
            signature: None,
            verified: None,
        });

        let ant_tp_config = AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let service = CryptoService::new(ant_tp_config);
        let result = service.sign_map(data_map);

        let crypto_struct = result.get(&data_hex).unwrap();
        assert!(crypto_struct.verified.unwrap());
        assert!(crypto_struct.signature.is_some());

        // Crypto the generated signature
        let is_verified = service.verify(
            &hex::encode(secret_key.public_key().to_bytes()),
            crypto_struct.signature.as_ref().unwrap(),
            &data_hex
        );
        assert!(is_verified);
    }
}
