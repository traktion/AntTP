use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{ScratchpadAddress, SecretKey};
use bytes::Bytes;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct UpdatePrivateScratchpadCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
}

impl UpdatePrivateScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64, data: Bytes) -> Self {
        Self { client_harness, owner, content_type, data }
    }
}

#[async_trait]
impl Command for UpdatePrivateScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };
        
        let scratchpad_address_hex = ScratchpadAddress::new(self.owner.public_key()).to_hex();
        debug!("updating private scratchpad at [{}] async", scratchpad_address_hex);
        match client.scratchpad_update(&self.owner, self.content_type, &self.data).await {
            Ok(_) => {
                info!("private scratchpad at address [{}] updated successfully", scratchpad_address_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::from(e.to_string()))
        }
    }
}