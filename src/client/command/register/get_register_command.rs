use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::register::RegisterAddress;
use foyer::HybridCache;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct GetRegisterCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    register_address: RegisterAddress,
    ttl: u64,
}

impl GetRegisterCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, register_address: RegisterAddress, ttl: u64) -> Self {
        Self { client_harness, hybrid_cache, register_address, ttl }
    }
}

#[async_trait]
impl Command for GetRegisterCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };
        let register_address_hex = self.register_address.to_hex();
        debug!("refreshing hybrid cache with register for [{}] from network", register_address_hex);
        match client.register_get(&self.register_address).await {
            Ok(register_value) => {
                let new_cache_item = CacheItem::new(Some(register_value.clone()), self.ttl);
                self.hybrid_cache.insert(
                    format!("rg{}", register_address_hex),
                    rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize register")
                );
                info!("refreshed hybrid cache with register for [{}] from network", register_address_hex);
                Ok(())
            }
            Err(e) => {
                Err(CommandError::from(
                    format!("Failed to refresh hybrid cache with register for [{}] from network [{}]", register_address_hex, e)))
            }
        }
    }
}