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

pub struct CreatePublicScratchpadCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
    payment_option: PaymentOption,
}

impl CreatePublicScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64,
               data: Bytes, payment_option: PaymentOption,) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, content_type, data, payment_option }
    }

    async fn scratchpad_check_existence(
        &self,
        address: &ScratchpadAddress,
    ) -> Result<bool, CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        Ok(client.scratchpad_check_existence(address).await.is_ok())
    }
}

const STRUCT_NAME: &'static str = "CreatePublicScratchpadCommand";

#[async_trait]
impl Command for CreatePublicScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let address = ScratchpadAddress::new(self.owner.public_key());
        if self.scratchpad_check_existence(&address).await? {
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
            client.scratchpad_put(scratchpad, self.payment_option.clone()).await?;
            info!("public scratchpad at address [{}] created successfully", address.to_hex());
            Ok(())
        }
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.owner.to_hex());
        hasher.update(self.content_type.to_string());
        hasher.update(self.data.clone());
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
        properties.insert("owner".to_string(), self.owner.to_hex());
        properties.insert("content_type".to_string(), self.content_type.to_string());
        properties.insert("data".to_string(), "tbc".to_string()); // todo: improve
        properties
    }
}