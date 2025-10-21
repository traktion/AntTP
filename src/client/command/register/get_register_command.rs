use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::register::RegisterAddress;
use foyer::HybridCache;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::cache_item::CacheItem;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;
use crate::client::REGISTER_CACHE_KEY;

pub struct GetRegisterCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    register_address: RegisterAddress,
    ttl: u64,
}

impl GetRegisterCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, hybrid_cache: Data<HybridCache<String, Vec<u8>>>, register_address: RegisterAddress, ttl: u64) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, hybrid_cache, register_address, ttl }
    }
}

const STRUCT_NAME: &'static str = "GetRegisterCommand";

#[async_trait]
impl Command for GetRegisterCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::Recoverable(String::from("network offline")))
        };
        let register_address_hex = self.register_address.to_hex();
        debug!("refreshing hybrid cache with register for [{}] from network", register_address_hex);
        match client.register_get(&self.register_address).await {
            Ok(register_value) => {
                let new_cache_item = CacheItem::new(Some(register_value.clone()), self.ttl);
                self.hybrid_cache.insert(
                    format!("{}{}", REGISTER_CACHE_KEY, register_address_hex),
                    rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize register")
                );
                info!("refreshed hybrid cache with register for [{}] from network", register_address_hex);
                Ok(())
            }
            Err(e) => {
                Err(CommandError::Unrecoverable(
                    format!("Failed to refresh hybrid cache with register for [{}] from network [{}]", register_address_hex, e)))
            }
        }
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.register_address.to_hex());
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
        properties.insert("register_address".to_string(), self.register_address.to_hex());
        properties
    }
}