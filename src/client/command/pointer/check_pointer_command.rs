use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{PointerAddress};
use foyer::HybridCache;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct CheckPointerCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    pointer_address: PointerAddress,
    ttl: u64,
}

impl CheckPointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, pointer_address: PointerAddress, ttl: u64) -> Self {
        Self { client_harness, hybrid_cache, pointer_address, ttl }
    }
}

#[async_trait]
impl Command for CheckPointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };
        
        let pointer_address_hex = self.pointer_address.to_hex();
        debug!("refreshing hybrid cache with pointer check existence for [{}] from network", pointer_address_hex);
        match client.pointer_check_existence(&self.pointer_address).await {
            Ok(is_pointer) => {
                let new_cache_item = CacheItem::new(Some(is_pointer.clone()), self.ttl);
                self.hybrid_cache.insert(
                    format!("pce{}", pointer_address_hex),
                    rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize pointer check existence")
                );
                info!("refreshed hybrid cache with pointer check existence for [{}] from network", pointer_address_hex);
                Ok(())
            }
            Err(e) => {
                Err(CommandError::from(
                    format!("Failed to refresh hybrid cache with pointer check existence for [{}] from network [{}]", pointer_address_hex, e)))
            }
        }
    }
}