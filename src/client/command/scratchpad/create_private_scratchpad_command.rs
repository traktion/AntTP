use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::SecretKey;
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::Command;
use crate::client::command::error::CommandError;

pub struct CreatePrivateScratchpadCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    content_type: u64,
    data: Bytes,
    payment_option: PaymentOption,
}

impl CreatePrivateScratchpadCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, content_type: u64,
               data: Bytes, payment_option: PaymentOption,) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, content_type, data, payment_option }
    }
}

const STRUCT_NAME: &'static str = "CreatePrivateScratchpadCommand";

#[async_trait]
impl Command for CreatePrivateScratchpadCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let scratchpad_address_hex = autonomi::ScratchpadAddress::new(self.owner.public_key()).to_hex();
        debug!("creating private scratchpad at [{}] async", scratchpad_address_hex);
        client.scratchpad_create(&self.owner, self.content_type, &self.data, self.payment_option.clone()).await?;
        info!("private scratchpad at address [{}] created successfully", scratchpad_address_hex);
        Ok(())
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