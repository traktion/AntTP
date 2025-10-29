use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{PointerAddress, SecretKey};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct CreatePointerCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    target: PointerTarget,
    payment_option: PaymentOption,
}

impl CreatePointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, target: PointerTarget,
               payment_option: PaymentOption,) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, target, payment_option }
    }
}

const STRUCT_NAME: &'static str = "CheckPointerCommand";

#[async_trait]
impl Command for CreatePointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let pointer_address_hex = PointerAddress::new(self.owner.public_key()).to_hex();
        debug!("creating pointer at [{}] async", pointer_address_hex);
        client.pointer_create(&self.owner, self.target.clone(), self.payment_option.clone()).await?;
        info!("pointer at address [{}] created successfully", pointer_address_hex);
        Ok(())
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.owner.to_hex());
        hasher.update(self.target.to_hex());
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
        properties.insert("target".to_string(), self.target.to_hex());
        properties
    }
}