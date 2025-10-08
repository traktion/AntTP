use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{Scratchpad, ScratchpadAddress, SecretKey};
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::{Command, CommandError};

pub struct UpdatePublicScratchpadCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
    payment_option: PaymentOption,
}

impl UpdatePublicScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64,
               data: Bytes, payment_option: PaymentOption) -> Self {
        Self { client_harness, owner, content_type, data, payment_option }
    }
}

#[async_trait]
impl Command for UpdatePublicScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };
        
        let address = ScratchpadAddress::new(self.owner.public_key());
        debug!("updating public scratchpad at [{}] async", address.to_hex());
        match client.scratchpad_get(&address).await {
            Ok(scratchpad) => {
                let version = scratchpad.counter() + 1;
                let signature = self.owner.sign(Scratchpad::bytes_for_signature(
                    address,
                    self.content_type,
                    &self.data.clone(),
                    version,
                ));
                let scratchpad = Scratchpad::new_with_signature(
                    self.owner.public_key(), self.content_type, self.data.clone(), version, signature);

                match client.scratchpad_put(scratchpad, self.payment_option.clone()).await {
                    Ok(_) => {
                        info!("public scratchpad at address [{}] updated successfully", address.to_hex());
                        Ok(())
                    },
                    Err(e) => Err(CommandError::from(e.to_string()))
                }
            },
            Err(e) => Err(CommandError::from(e.to_string()))
        }
    }
}