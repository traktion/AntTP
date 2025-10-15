use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{PointerAddress, SecretKey};
use autonomi::pointer::PointerTarget;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct UpdatePointerCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    target: PointerTarget
}

impl UpdatePointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, target: PointerTarget) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, target }
    }
}

const STRUCT_NAME: &'static str = "UpdatePointerCommand";

#[async_trait]
impl Command for UpdatePointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::Recoverable(String::from("network offline")))
        };

        let pointer_address_hex = PointerAddress::new(self.owner.public_key()).to_hex();
        debug!("updating pointer at [{}] async", pointer_address_hex);
        match client.pointer_update(&self.owner, self.target.clone()).await {
            Ok(_) => {
                info!("pointer at address [{}] updated successfully", pointer_address_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::Unrecoverable(
                format!("Failed to update pointer for [{}] on network [{}]", pointer_address_hex, e)))
        }
    }

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.owner.to_hex());
        hasher.update(self.target.to_hex());
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
        properties.insert("owner".to_string(), self.owner.to_hex());
        properties.insert("target".to_string(), self.target.to_hex());
        properties
    }
}