use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{Scratchpad, ScratchpadAddress, SecretKey};
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct CreatePublicScratchpadCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
    payment_option: PaymentOption,
}

impl CreatePublicScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64,
               data: Bytes, payment_option: PaymentOption,) -> Self {
        Self { client_harness, owner, content_type, data, payment_option }
    }

    async fn scratchpad_check_existence(
        &self,
        address: &ScratchpadAddress,
    ) -> bool {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client.scratchpad_check_existence(address).await.is_ok(),
            None => false,
        }
    }
}

#[async_trait]
impl Command for CreatePublicScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        let address = ScratchpadAddress::new(self.owner.public_key());
        if self.scratchpad_check_existence(&address).await {
            Ok(())
        } else {
            let counter = 0;
            let signature = self.owner.sign(Scratchpad::bytes_for_signature(
                address,
                self.content_type,
                &self.data.clone(),
                counter,
            ));
            // create an _unencrypted_ scratchpad
            let scratchpad = Scratchpad::new_with_signature(
                self.owner.public_key(), self.content_type, self.data.clone(), counter, signature);

            debug!("creating public scratchpad at [{}] async", address.to_hex());
            match client.scratchpad_put(scratchpad, self.payment_option.clone()).await {
                Ok(_) => {
                    info!("public scratchpad at address [{}] created successfully", address.to_hex());
                    Ok(())
                },
                Err(e) => Err(CommandError::from(e.to_string()))
            }
        }
    }

    fn get_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update("CreatePublicScratchpadCommand");
        hasher.update(self.owner.to_hex());
        hasher.update(self.content_type.to_string());
        hasher.update(self.data.clone());
        hasher.finalize().to_ascii_lowercase()
    }
}