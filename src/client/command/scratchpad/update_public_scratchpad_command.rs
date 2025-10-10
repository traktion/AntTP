use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{Scratchpad, ScratchpadAddress, SecretKey};
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct UpdatePublicScratchpadCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
    payment_option: PaymentOption,
}

impl UpdatePublicScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64,
               data: Bytes, payment_option: PaymentOption) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, content_type, data, payment_option }
    }
}

const STRUCT_NAME: &'static str = "UpdatePublicScratchpadCommand";

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

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.owner.to_hex());
        hasher.update(self.content_type.to_string());
        hasher.update(self.data.clone());
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
        properties.insert("content_type".to_string(), self.content_type.to_string());
        properties.insert("data".to_string(), "tbc".to_string()); // todo: improve
        properties
    }
}