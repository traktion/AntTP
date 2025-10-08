use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::ScratchpadAddress;
use foyer::HybridCache;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct GetScratchpadCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    scratchpad_address: ScratchpadAddress,
    ttl: u64,
}

impl GetScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, scratchpad_address: ScratchpadAddress, ttl: u64) -> Self {
        Self { client_harness, hybrid_cache, scratchpad_address, ttl }
    }
}

#[async_trait]
impl Command for GetScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };
        
        let scratchpad_address_hex = self.scratchpad_address.to_hex();
        debug!("refreshing hybrid cache with scratchpad for [{}] from network", scratchpad_address_hex);
        match client.scratchpad_get(&self.scratchpad_address).await {
            Ok(scratchpad) => {
                let new_cache_item = CacheItem::new(Some(scratchpad.clone()), self.ttl);
                self.hybrid_cache.insert(
                    format!("sg{}", scratchpad_address_hex),
                    rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize scratchpad")
                );
                info!("refreshed hybrid cache with scratchpad for [{}] from network", scratchpad_address_hex);
                Ok(())
            }
            Err(e) => {
                Err(CommandError::from(
                    format!("Failed to refresh hybrid cache with scratchpad for [{}] from network [{}]", scratchpad_address_hex, e)))
            }
        }
    }
}