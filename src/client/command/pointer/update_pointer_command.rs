use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{PointerAddress, SecretKey};
use autonomi::pointer::PointerTarget;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct UpdatePointerCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    target: PointerTarget
}

impl UpdatePointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, target: PointerTarget) -> Self {
        Self { client_harness, owner, target }
    }
}

#[async_trait]
impl Command for UpdatePointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };
        
        let pointer_address_hex = PointerAddress::new(self.owner.public_key()).to_hex();
        debug!("updating pointer at [{}] async", pointer_address_hex);
        match client.pointer_update(&self.owner, self.target.clone()).await {
            Ok(_) => {
                info!("pointer at address [{}] updated successfully", pointer_address_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::from(
                format!("Failed to update pointer for [{}] on network [{}]", pointer_address_hex, e)))
        }
    }
}