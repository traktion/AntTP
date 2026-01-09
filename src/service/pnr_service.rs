use crate::client::CachingClient;
use crate::controller::{CacheType, DataKey};
use crate::error::pointer_error::PointerError;
use crate::error::CreateError;
use crate::model::pnr::PnrZone;
use crate::service::pointer_service::{Pointer, PointerService};
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

    pub async fn create_pnr(&self, pnr_zone: PnrZone, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<PnrZone, PointerError> {
        /*
        1. Create chunk containing PNR zone (container for records)
        2. Create mutable personal pointer to above chunk
        3. Create immutable (TTL=MAX) resolver pointer to personal pointer
         */
        match self.caching_client.chunk_put(
            &Chunk::new(Bytes::from(serde_json::to_vec(&pnr_zone).unwrap())),
            PaymentOption::from(&evm_wallet),
            cache_only.clone()
        ).await {
            Ok(chunk) => {
                let personal_pointer_request = Pointer::new(
                    Some(pnr_zone.name.clone()), chunk.to_hex(), None, None, None,
                );
                match self.pointer_service.create_pointer(
                    personal_pointer_request,
                    evm_wallet.clone(),
                    cache_only.clone(),
                    DataKey::Personal).await
                {
                    Ok(personal_pointer_result) => {
                        let resolver_pointer_request = Pointer::new(
                            Some(pnr_zone.name.clone()), personal_pointer_result.address.clone().unwrap(), None, None, None,
                        );
                        match self.pointer_service.create_pointer(
                            resolver_pointer_request,
                            evm_wallet,
                            cache_only,
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
}