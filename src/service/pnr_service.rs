use crate::client::ChunkCachingClient;
use crate::controller::{DataKey, StoreType};
use crate::error::pointer_error::PointerError;
use crate::error::{CreateError, UpdateError};
use crate::model::pnr::{PnrRecord, PnrZone};
use crate::service::pointer_service::{Pointer, PointerService};
use actix_web::web::Data;
use ant_protocol::storage::{Chunk, ChunkAddress};
use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use bytes::Bytes;

use crate::service::{validate_immutable_address, validate_immutable_addresses};

#[derive(Debug, Clone)]
pub struct PnrService {
    chunk_caching_client: ChunkCachingClient,
    pointer_service: Data<PointerService>
}

impl PnrService {
    pub fn new(chunk_caching_client: ChunkCachingClient, pointer_service: Data<PointerService>) -> Self {
        Self { chunk_caching_client, pointer_service }
    }

    pub async fn create_mutable_pnr(&self, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        /*
        1. Create chunk containing PNR zone (container for records)
        2. Create mutable personal pointer to above chunk
        3. Create immutable (TTL=MAX) resolver pointer to personal pointer
        4. Trim whitespace from PNR zone name
         */
        pnr_zone.name = pnr_zone.name.trim().to_string();
        match self.chunk_caching_client.chunk_put(
            &Chunk::new(Bytes::from(serde_json::to_vec(&pnr_zone).unwrap())),
            PaymentOption::from(&evm_wallet),
            store_type.clone()
        ).await {
            Ok(chunk) => {
                let personal_pointer_request = Pointer::new(
                    Some(pnr_zone.name.clone()), chunk.to_hex(), None, None, None,
                );
                match self.pointer_service.create_pointer(
                    personal_pointer_request,
                    evm_wallet.clone(),
                    store_type.clone(),
                    DataKey::Personal).await
                {
                    Ok(personal_pointer_result) => {
                        let resolver_pointer_request = Pointer::new(
                            Some(pnr_zone.name.clone()), personal_pointer_result.address.clone().unwrap(), None, Some(u64::MAX), None,
                        );
                        match self.pointer_service.create_pointer(
                            resolver_pointer_request,
                            evm_wallet,
                            store_type,
                            DataKey::Resolver).await
                        {
                            Ok(resolver_pointer_result) => {
                                Ok(PnrZone::new(
                                    pnr_zone.name.clone(),
                                    pnr_zone.records.clone(),
                                    resolver_pointer_result.address.clone(),
                                    personal_pointer_result.address.clone(),
                                ))
                            },
                            Err(e) => Err(e),
                        }
                    },
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(PointerError::CreateError(CreateError::InvalidData(e.to_string())))
        }
    }

    pub async fn create_immutable_pnr(&self, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        /*
        1. Create chunk containing PNR zone (container for records)
        2. Create immutable (TTL=MAX) resolver pointer to chunk directly
        3. Trim whitespace from PNR zone name
        4. Validate all record addresses are immutable XOR addresses (64-character hex strings)
         */
        pnr_zone.name = pnr_zone.name.trim().to_string();
        validate_immutable_addresses(&pnr_zone.records)?;

        match self.chunk_caching_client.chunk_put(
            &Chunk::new(Bytes::from(serde_json::to_vec(&pnr_zone).unwrap())),
            PaymentOption::from(&evm_wallet),
            store_type.clone()
        ).await {
            Ok(chunk) => {
                let resolver_pointer_request = Pointer::new(
                    Some(pnr_zone.name.clone()), chunk.to_hex(), None, Some(u64::MAX), None,
                );
                match self.pointer_service.create_pointer(
                    resolver_pointer_request,
                    evm_wallet,
                    store_type,
                    DataKey::Resolver).await
                {
                    Ok(resolver_pointer_result) => {
                        Ok(PnrZone::new(
                            pnr_zone.name.clone(),
                            pnr_zone.records.clone(),
                            resolver_pointer_result.address.clone(),
                            None,
                        ))
                    },
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(PointerError::CreateError(CreateError::InvalidData(e.to_string())))
        }
    }

    pub async fn update_pnr(&self, name: String, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        let name = name.trim().to_string();
        pnr_zone.name = pnr_zone.name.trim().to_string();

        let (resolver_address, personal_pointer_address, is_immutable) = self.resolve_pnr_address(&name).await?;

        if is_immutable {
            return Err(PointerError::UpdateError(UpdateError::InvalidData("Cannot update an immutable PNR zone".to_string())));
        }

        match self.chunk_caching_client.chunk_put(
            &Chunk::new(Bytes::from(serde_json::to_vec(&pnr_zone).unwrap())),
            PaymentOption::from(&evm_wallet),
            store_type.clone()
        ).await {
            Ok(chunk) => {
                let personal_pointer_request = Pointer::new(
                    Some(name.clone()), chunk.to_hex(), None, None, None,
                );
                match self.pointer_service.update_pointer(
                    personal_pointer_address.clone(),
                    personal_pointer_request,
                    store_type,
                    DataKey::Personal).await
                {
                    Ok(_) => {
                        Ok(PnrZone::new(
                            name,
                            pnr_zone.records,
                            Some(resolver_address),
                            Some(personal_pointer_address),
                        ))
                    },
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(PointerError::UpdateError(UpdateError::InvalidData(e.to_string())))
        }
    }

    pub async fn get_pnr(&self, name: String) -> Result<PnrZone, PointerError> {
        let name = name.trim().to_string();
        let (resolver_address, content_address, is_immutable) = self.resolve_pnr_address(&name).await?;

        let pnr_zone_address = if is_immutable {
            content_address.clone()
        } else {
            let personal_pointer = self.pointer_service.get_pointer(content_address.clone(), DataKey::Personal).await?;
            personal_pointer.content
        };

        match self.chunk_caching_client.chunk_get_internal(&ant_protocol::storage::ChunkAddress::from_hex(&pnr_zone_address)?).await {
            Ok(chunk) => {
                let mut pnr_zone: PnrZone = serde_json::from_slice(chunk.value.as_ref())
                    .map_err(|e| PointerError::UpdateError(UpdateError::InvalidData(e.to_string())))?;

                if is_immutable {
                    pnr_zone.records.retain(|key, record| {
                        if let Err(e) = validate_immutable_address(&record.address) {
                            log::warn!("Removing invalid immutable address for record '{}' in immutable PNR zone '{}': {}", key, name, e);
                            false
                        } else {
                            true
                        }
                    });
                }

                Ok(PnrZone::new(
                    name,
                    pnr_zone.records,
                    Some(resolver_address),
                    if is_immutable { None } else { Some(content_address) },
                ))
            },
            Err(e) => Err(PointerError::UpdateError(UpdateError::InvalidData(e.to_string())))
        }
    }

    pub async fn append_pnr(&self, name: String, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        let name = name.trim().to_string();
        pnr_zone.name = pnr_zone.name.trim().to_string();

        let mut existing_pnr_zone = self.get_pnr(name.clone()).await?;

        // If the existing zone is immutable (no personal pointer), we shouldn't be able to append to it
        // but get_pnr will fail or return personal_address as None.
        // The issue only mentions validation when creating an immutable PNR.
        // However, if an immutable PNR zone chunk is retrieved and used here, we should still be careful.

        for (key, record) in pnr_zone.records {
            existing_pnr_zone.records.insert(key.trim().to_string(), record);
        }

        self.update_pnr(name, existing_pnr_zone, evm_wallet, store_type).await
    }

    pub async fn update_pnr_record(&self, name: String, record_key: String, record: PnrRecord, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        let name = name.trim().to_string();
        let record_key = record_key.trim().to_string();

        let mut existing_pnr_zone = self.get_pnr(name.clone()).await?;

        existing_pnr_zone.records.insert(record_key, record);

        self.update_pnr(name, existing_pnr_zone, evm_wallet, store_type).await
    }

    async fn resolve_pnr_address(&self, name: &String) -> Result<(String, String, bool), PointerError> {
        let resolver_address = self.pointer_service.get_resolver_address(name)?;
        let resolver_pointer = self.pointer_service.get_pointer(resolver_address.clone(), DataKey::Resolver).await?;
        
        let content_address = resolver_pointer.content;
        let is_immutable = ChunkAddress::from_hex(&content_address).is_ok();
        
        Ok((resolver_address, content_address, is_immutable))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::pnr::{PnrRecord, PnrRecordType};
    use ant_protocol::storage::ChunkAddress;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_append_pnr_logic() {
        // This is a logic-only test for the record merging, as full mocking of PnrService's dependencies 
        // (ChunkCachingClient and PointerService) for an integration test here is complex.
        
        let mut existing_records = HashMap::new();
        existing_records.insert("old".to_string(), PnrRecord::new("addr1".to_string(), PnrRecordType::A, 60));
        existing_records.insert("keep".to_string(), PnrRecord::new("addr2".to_string(), PnrRecordType::A, 60));
        
        let mut new_records = HashMap::new();
        new_records.insert("old".to_string(), PnrRecord::new("addr3".to_string(), PnrRecordType::X, 120));
        new_records.insert("new".to_string(), PnrRecord::new("addr4".to_string(), PnrRecordType::A, 60));

        // Simulate merging logic from append_pnr
        let mut merged_records = existing_records;
        for (key, record) in new_records {
            merged_records.insert(key, record);
        }

        assert_eq!(merged_records.len(), 3);
        assert_eq!(merged_records.get("old").unwrap().address, "addr3");
        assert_eq!(merged_records.get("old").unwrap().ttl, 120);
        assert!(matches!(merged_records.get("old").unwrap().record_type, PnrRecordType::X));
        assert_eq!(merged_records.get("keep").unwrap().address, "addr2");
        assert_eq!(merged_records.get("new").unwrap().address, "addr4");
    }

    #[test]
    fn test_validate_immutable_addresses_get_pnr() {
        use super::*;
        let mut records = HashMap::new();
        // Valid 64-char hex address
        let valid_addr = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();
        records.insert("valid".to_string(), PnrRecord::new(valid_addr.clone(), PnrRecordType::A, 60));
        // Invalid address (too short)
        records.insert("invalid".to_string(), PnrRecord::new("short".to_string(), PnrRecordType::A, 60));

        // Simulate the retain logic in get_pnr
        records.retain(|_, record| {
            validate_immutable_address(&record.address).is_ok()
        });

        assert_eq!(records.len(), 1);
        assert!(records.contains_key("valid"));
        assert!(!records.contains_key("invalid"));
    }

    #[tokio::test]
    async fn test_update_pnr_record_logic() {
        let mut existing_records = HashMap::new();
        existing_records.insert("keep".to_string(), PnrRecord::new("addr1".to_string(), PnrRecordType::A, 60));
        existing_records.insert("update".to_string(), PnrRecord::new("addr2".to_string(), PnrRecordType::A, 60));

        let new_record = PnrRecord::new("addr3".to_string(), PnrRecordType::X, 120);

        // Simulate merging logic from update_pnr_record
        let mut merged_records = existing_records;
        merged_records.insert("update".to_string(), new_record);

        assert_eq!(merged_records.len(), 2);
        assert_eq!(merged_records.get("update").unwrap().address, "addr3");
        assert_eq!(merged_records.get("update").unwrap().ttl, 120);
        assert!(matches!(merged_records.get("update").unwrap().record_type, PnrRecordType::X));
        assert_eq!(merged_records.get("keep").unwrap().address, "addr1");
    }

    #[test]
    fn test_validate_immutable_addresses() {
        use super::*;
        use crate::model::pnr::{PnrRecord, PnrRecordType};
        use std::collections::HashMap;

        let mut valid_records = HashMap::new();
        valid_records.insert("valid".to_string(), PnrRecord::new("a".repeat(64), PnrRecordType::A, 60));
        assert!(validate_immutable_addresses(&valid_records).is_ok());

        let mut too_short = HashMap::new();
        too_short.insert("invalid".to_string(), PnrRecord::new("a".repeat(63), PnrRecordType::A, 60));
        assert!(validate_immutable_addresses(&too_short).is_err());

        let mut non_hex = HashMap::new();
        non_hex.insert("invalid".to_string(), PnrRecord::new("g".repeat(64), PnrRecordType::A, 60));
        assert!(validate_immutable_addresses(&non_hex).is_err());
    }
}
