use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::PointerAddress;
use foyer::HybridCache;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct GetPointerCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    pointer_address: PointerAddress,
    ttl: u64,
}

impl GetPointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, pointer_address: PointerAddress, ttl: u64) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, hybrid_cache, pointer_address, ttl }
    }
}

const STRUCT_NAME: &'static str = "GetPointerCommand";

#[async_trait]
impl Command for GetPointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        let pointer_address_hex = self.pointer_address.to_hex();
        debug!("refreshing hybrid cache with pointer for [{}] from network", pointer_address_hex);
        match client.pointer_get(&self.pointer_address).await {
            Ok(pointer) => {
                let new_cache_item = CacheItem::new(Some(pointer.clone()), self.ttl);
                self.hybrid_cache.insert(
                    format!("pg{}", pointer_address_hex),
                    rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize pointer")
                );
                info!("refreshed hybrid cache with pointer for [{}] from network", pointer_address_hex);
                Ok(())
            },
            Err(e) => {
                Err(CommandError::from(
                    format!("Failed to refresh hybrid cache with pointer for [{}] from network [{}]", pointer_address_hex, e)))
            }
        }
    }

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME.to_string());
        hasher.update(self.pointer_address.to_hex());
        hasher.finalize().to_ascii_lowercase()
    }

    fn get_id(&self) -> u128 {
        self.id
    }

    fn get_name(&self) -> String {
        STRUCT_NAME.to_string()
    }

    fn get_properties(&self) -> IndexMap<String, String> {
        let mut properties = IndexMap::new();
        properties.insert("pointer_address".to_string(), self.pointer_address.to_hex());
        properties
    }
}