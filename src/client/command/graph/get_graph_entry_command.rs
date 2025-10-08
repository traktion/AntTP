use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::GraphEntryAddress;
use foyer::HybridCache;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct GetGraphEntryCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    graph_entry_address: GraphEntryAddress,
    ttl: u64,
}

impl GetGraphEntryCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, graph_entry_address: GraphEntryAddress, ttl: u64) -> Self {
        Self { client_harness, hybrid_cache, graph_entry_address, ttl }
    }
}

#[async_trait]
impl Command for GetGraphEntryCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        let graph_entry_address_hex = self.graph_entry_address.to_hex();
        debug!("refreshing hybrid cache with graph_entry for [{}] from network", graph_entry_address_hex);
        match client.graph_entry_get(&self.graph_entry_address).await {
            Ok(graph_entry) => {
                let new_cache_item = CacheItem::new(Some(graph_entry.clone()), self.ttl);
                self.hybrid_cache.insert(
                    format!("gg{}", graph_entry_address_hex),
                    rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize graph entry")
                );
                info!("refreshed hybrid cache with graph entry for [{}] from network", graph_entry_address_hex);
                Ok(())
            },
            Err(e) => {
                Err(CommandError::from(
                    format!("Failed to refresh hybrid cache with graph entry for [{}] from network [{}]", graph_entry_address_hex, e)))
            }
        }
    }

    fn get_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update("GetGraphEntryCommand");
        hasher.update(self.graph_entry_address.to_hex());
        hasher.finalize().to_ascii_lowercase()
    }
}