use blsttc::{PublicKey, Signature};
use base64::{engine::general_purpose, Engine as _};

#[derive(Debug)]
pub struct SignatureService;

impl SignatureService {
    pub fn verify(&self, public_key_bytes: &[u8], signature_bytes: &[u8], data: &[u8]) -> bool {
        let mut pk_arr = [0u8; 48];
        if public_key_bytes.len() != 48 {
            return false;
        }
        pk_arr.copy_from_slice(public_key_bytes);
        let public_key = match PublicKey::from_bytes(pk_arr) {
            Ok(pk) => pk,
            Err(_) => return false,
        };

        let mut sig_arr = [0u8; 96];
        if signature_bytes.len() != 96 {
            return false;
        }
        sig_arr.copy_from_slice(signature_bytes);
        let signature = match Signature::from_bytes(sig_arr) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        public_key.verify(&signature, data)
    }

    pub fn verify_hex(&self, public_key_hex: &str, signature_hex: &str, data_hex: &str) -> bool {
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

        self.verify(&public_key_bytes, &signature_bytes, &data_bytes)
    }

    pub fn decode_base64(encoded: &str) -> Option<Vec<u8>> {
        general_purpose::STANDARD.decode(encoded).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blsttc::SecretKey;

    #[test]
    fn test_verify_signature() {
        let secret_key = SecretKey::random();
        let public_key = secret_key.public_key();
        let data = b"hello world";
        let signature = secret_key.sign(data);

        let signature_service = SignatureService;
        let is_verified = signature_service.verify(
            &public_key.to_bytes(),
            &signature.to_bytes(),
            data
        );

        assert!(is_verified);
    }

    #[test]
    fn test_verify_invalid_signature() {
        let secret_key = SecretKey::random();
        let public_key = secret_key.public_key();
        let data = b"hello world";
        let signature = secret_key.sign(b"other data");

        let signature_service = SignatureService;
        let is_verified = signature_service.verify(
            &public_key.to_bytes(),
            &signature.to_bytes(),
            data
        );

        assert!(!is_verified);
    }

    #[test]
    fn test_decode_base64() {
        let data = b"hello world";
        let encoded = general_purpose::STANDARD.encode(data);
        let decoded = SignatureService::decode_base64(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_verify_hex_signature() {
        let secret_key = SecretKey::random();
        let public_key = secret_key.public_key();
        let data = b"hello world";
        let signature = secret_key.sign(data);

        let signature_service = SignatureService;
        let is_verified = signature_service.verify_hex(
            &hex::encode(public_key.to_bytes()),
            &hex::encode(signature.to_bytes()),
            &hex::encode(data)
        );

        assert!(is_verified);
    }

    #[test]
    fn test_verify_hex_invalid_signature() {
        let secret_key = SecretKey::random();
        let public_key = secret_key.public_key();
        let data = b"hello world";
        let signature = secret_key.sign(b"other data");

        let signature_service = SignatureService;
        let is_verified = signature_service.verify_hex(
            &hex::encode(public_key.to_bytes()),
            &hex::encode(signature.to_bytes()),
            &hex::encode(data)
        );

        assert!(!is_verified);
    }
}
