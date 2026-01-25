use crate::client::{CachingClient, ChunkCachingClient};
use crate::controller::{StoreType, DataKey};
use crate::error::pointer_error::PointerError;
use crate::error::CreateError;
use crate::model::pnr::PnrZone;
use crate::service::pointer_service::{Pointer, PointerService};
use crate::error::UpdateError;
use actix_web::web::Data;
use ant_protocol::storage::Chunk;
use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct PnrService {
    caching_client: CachingClient,
    pointer_service: Data<PointerService>
}

impl PnrService {
    pub fn new(caching_client: CachingClient, pointer_service: Data<PointerService>) -> Self {
        Self { caching_client, pointer_service }
    }

    pub async fn create_pnr(&self, pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        /*
        1. Create chunk containing PNR zone (container for records)
        2. Create mutable personal pointer to above chunk
        3. Create immutable (TTL=MAX) resolver pointer to personal pointer
         */
        match ChunkCachingClient::new(self.caching_client.clone()).chunk_put(
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
                            Some(pnr_zone.name.clone()), personal_pointer_result.address.clone().unwrap(), None, None, None,
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

    pub async fn update_pnr(&self, name: String, pnr_zone: PnrZone, evm_wallet: Wallet, store_type: StoreType) -> Result<PnrZone, PointerError> {
        let (resolver_address, personal_pointer_address) = self.resolve_personal_address(&name).await?;

        match ChunkCachingClient::new(self.caching_client.clone()).chunk_put(
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
        let (resolver_address, personal_pointer_address) = self.resolve_personal_address(&name).await?;

        let personal_pointer = self.pointer_service.get_pointer(personal_pointer_address.clone()).await?;
        let pnr_zone_address = personal_pointer.content;

        match ChunkCachingClient::new(self.caching_client.clone()).chunk_get_internal(&ant_protocol::storage::ChunkAddress::from_hex(&pnr_zone_address)?).await {
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

    async fn resolve_personal_address(&self, name: &String) -> Result<(String, String), PointerError> {
        let resolver_address = self.pointer_service.get_resolver_address(name)?;
        let personal_pointer_address = match self.pointer_service.get_pointer(resolver_address.clone()).await {
            Ok(pointer) => pointer.content,
            Err(e) => return Err(e),
        };
        Ok((resolver_address, personal_pointer_address))
    }
}
