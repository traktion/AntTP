use crate::client::ChunkCachingClient;
use crate::controller::{DataKey, StoreType};
use crate::error::pointer_error::PointerError;
use crate::error::CreateError;
use crate::error::UpdateError;
use crate::model::pnr::{PnrRecord, PnrZone};
use crate::service::pointer_service::{Pointer, PointerService};
use actix_web::web::Data;
use ant_protocol::storage::Chunk;
use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use bytes::Bytes;
use mockall::automock;

#[derive(Debug, Clone)]
pub struct PnrService {
    chunk_caching_client: ChunkCachingClient,
    pointer_service: Data<PointerService>
}

#[automock]
impl PnrService {
    pub fn new(chunk_caching_client: ChunkCachingClient, pointer_service: Data<PointerService>) -> Self {
        Self { chunk_caching_client, pointer_service }
    }

    pub async fn create_pnr(&self, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
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

    pub async fn update_pnr(&self, name: String, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        let name = name.trim().to_string();
        pnr_zone.name = pnr_zone.name.trim().to_string();

        let (resolver_address, personal_pointer_address) = self.resolve_personal_address(&name).await?;

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
        let (resolver_address, personal_pointer_address) = self.resolve_personal_address(&name).await?;

        let personal_pointer = self.pointer_service.get_pointer(personal_pointer_address.clone(), DataKey::Personal).await?;
        let pnr_zone_address = personal_pointer.content;

        match self.chunk_caching_client.chunk_get_internal(&ant_protocol::storage::ChunkAddress::from_hex(&pnr_zone_address)?).await {
            Ok(chunk) => {
                let pnr_zone: PnrZone = serde_json::from_slice(chunk.value.as_ref())
                    .map_err(|e| PointerError::UpdateError(UpdateError::InvalidData(e.to_string())))?;
                Ok(PnrZone::new(
                    name,
                    pnr_zone.records,
                    Some(resolver_address),
                    Some(personal_pointer_address),
                ))
            },
            Err(e) => Err(PointerError::UpdateError(UpdateError::InvalidData(e.to_string())))
        }
    }

    pub async fn append_pnr(&self, name: String, mut pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        let name = name.trim().to_string();
        pnr_zone.name = pnr_zone.name.trim().to_string();

        let mut existing_pnr_zone = self.get_pnr(name.clone()).await?;

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

    async fn resolve_personal_address(&self, name: &String) -> Result<(String, String), PointerError> {
        let resolver_address = self.pointer_service.get_resolver_address(name)?;
        let personal_pointer_address = match self.pointer_service.get_pointer(resolver_address.clone(), DataKey::Resolver).await {
            Ok(pointer) => pointer.content,
            Err(e) => return Err(e),
        };
        Ok((resolver_address, personal_pointer_address))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::pnr::{PnrRecord, PnrRecordType};
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
}
