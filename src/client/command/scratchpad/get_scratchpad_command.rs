use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::ScratchpadAddress;
use foyer::HybridCache;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;
use crate::client::SCRATCHPAD_CACHE_KEY;

pub struct GetScratchpadCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    scratchpad_address: ScratchpadAddress,
    ttl: u64,
}

impl GetScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, scratchpad_address: ScratchpadAddress, ttl: u64) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, hybrid_cache, scratchpad_address, ttl }
    }
}

const STRUCT_NAME: &'static str = "GetScratchpadCommand";

#[async_trait]
impl Command for GetScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let scratchpad_address_hex = self.scratchpad_address.to_hex();
        debug!("refreshing hybrid cache with scratchpad for [{}] from network", scratchpad_address_hex);
        let scratchpad = client.scratchpad_get(&self.scratchpad_address).await?;
        let new_cache_item = CacheItem::new(Some(scratchpad.clone()), self.ttl);
        self.hybrid_cache.insert(
            format!("{}{}", SCRATCHPAD_CACHE_KEY, scratchpad_address_hex),
            rmp_serde::to_vec(&new_cache_item)?
        );
        info!("refreshed hybrid cache with scratchpad for [{}] from network", scratchpad_address_hex);
        Ok(())
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.scratchpad_address.to_hex());
        hasher.finalize().to_ascii_lowercase()
    }

    fn id(&self) -> u128 {
        self.id
    }

    fn name(&self) -> String {
        STRUCT_NAME.to_string()
    }

    fn properties(&self) -> IndexMap<String, String> {
        let mut properties = IndexMap::new();
        properties.insert("scratchpad_address".to_string(), self.scratchpad_address.to_hex());
        properties
    }
}
