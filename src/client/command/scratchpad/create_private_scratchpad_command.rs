use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{SecretKey};
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct CreatePrivateScratchpadCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
    payment_option: PaymentOption,
}

impl CreatePrivateScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64,
               data: Bytes, payment_option: PaymentOption,) -> Self {
        Self { client_harness, owner, content_type, data, payment_option }
    }
}

#[async_trait]
impl Command for CreatePrivateScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        let scratchpad_address_hex = autonomi::ScratchpadAddress::new(self.owner.public_key()).to_hex();
        debug!("creating private scratchpad at [{}] async", scratchpad_address_hex);
        match client.scratchpad_create(&self.owner, self.content_type, &self.data, self.payment_option.clone()).await {
            Ok(_) => {
                info!("private scratchpad at address [{}] created successfully", scratchpad_address_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::from(e.to_string()))
        }
    }

    fn get_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update("CreatePrivateScratchpadCommand");
        hasher.update(self.owner.to_hex());
        hasher.update(self.content_type.to_string());
        hasher.update(self.data.clone());
        hasher.finalize().to_ascii_lowercase()
    }
}