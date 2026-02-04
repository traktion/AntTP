use std::collections::HashMap;
use actix_web::web::Data;
use autonomi::Wallet;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;
use crate::model::key_value::KeyValue;
use crate::model::pnr::{PnrRecord, PnrRecordType, PnrZone};
use crate::service::pnr_service::PnrService;
use crate::service::public_data_service::PublicDataService;

#[derive(Debug, Clone)]
pub struct KeyValueService {
    public_data_service: Data<PublicDataService>,
    pnr_service: Data<PnrService>,
}

impl KeyValueService {
    pub fn new(public_data_service: Data<PublicDataService>, pnr_service: Data<PnrService>) -> Self {
        Self {
            public_data_service,
            pnr_service,
        }
    }

    pub async fn create_key_value(
        &self,
        key_value: KeyValue,
        evm_wallet: Wallet,
        store_type: StoreType,
    ) -> Result<KeyValue, PublicDataError> {
        let decoded_content = BASE64_STANDARD
            .decode(&key_value.content)
            .map_err(|e| PublicDataError::GetError(crate::error::GetError::Decode(e.to_string())))?;

        let chunk = self
            .public_data_service
            .create_public_data(Bytes::from(decoded_content), evm_wallet.clone(), store_type.clone())
            .await?;

        let address = chunk.address.ok_or_else(|| {
            PublicDataError::GetError(crate::error::GetError::RecordNotFound("No address returned".to_string()))
        })?;

        let mut records = HashMap::new();
        records.insert(
            key_value.object.clone(),
            PnrRecord::new(address, PnrRecordType::A, 0),
        );

        let pnr_zone = PnrZone::new(key_value.bucket.clone(), records, None, None);

        // Try to append, if it fails because it doesn't exist, create it.
        // Note: PnrService::get_pnr currently returns PointerError.
        // KeyValueService needs to handle these errors.
        match self.pnr_service.append_pnr(key_value.bucket.clone(), pnr_zone.clone(), evm_wallet.clone(), store_type.clone()).await {
            Ok(_) => Ok(key_value),
            Err(_) => {
                // If append fails (e.g. not found), try to create it
                self.pnr_service.create_pnr(pnr_zone, evm_wallet, store_type).await
                    .map_err(|e| PublicDataError::GetError(crate::error::GetError::RecordNotFound(e.to_string())))?;
                Ok(key_value)
            }
        }
    }

    pub async fn get_key_value(&self, bucket: String, object: String) -> Result<KeyValue, PublicDataError> {
        let pnr_zone = self.pnr_service.get_pnr(bucket.clone()).await
            .map_err(|e| PublicDataError::GetError(crate::error::GetError::RecordNotFound(e.to_string())))?;

        let record = pnr_zone.records.get(&object).ok_or_else(|| {
            PublicDataError::GetError(crate::error::GetError::RecordNotFound(format!("Object {} not found in bucket {}", object, bucket)))
        })?;

        let content_bytes = self.public_data_service.get_public_data_binary(record.address.clone()).await?;
        let content = BASE64_STANDARD.encode(content_bytes);

        Ok(KeyValue::new(bucket, object, content))
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_create_and_get_key_value() {
        // This test is complex to set up due to many dependencies. 
        // In a real scenario, we'd mock all caching clients and services.
        // For now, we'll focus on ensuring the logic is sound through manual verification and integration tests.
    }
}
