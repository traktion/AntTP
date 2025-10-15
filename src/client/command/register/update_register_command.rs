use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::SecretKey;
use autonomi::client::payment::PaymentOption;
use autonomi::register::{RegisterAddress, RegisterValue};
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct UpdateRegisterCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    register_value: RegisterValue,
    payment_option: PaymentOption,
}

impl UpdateRegisterCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, register_value: RegisterValue,
               payment_option: PaymentOption,) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, register_value, payment_option }
    }
}

const STRUCT_NAME: &'static str = "UpdateRegisterCommand";

#[async_trait]
impl Command for UpdateRegisterCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::Recoverable(String::from("network offline")))
        };

        let register_address_hex = RegisterAddress::new(self.owner.public_key()).to_hex();
        debug!("updating register at [{}] async", register_address_hex);
        match client.register_update(&self.owner, self.register_value, self.payment_option.clone()).await {
            Ok(_) => {
                info!("register at address [{}] updated successfully", register_address_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::Unrecoverable(e.to_string()))
        }
    }

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.owner.to_hex());
        hasher.update(self.register_value.clone());
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
        properties.insert("register_value".to_string(), "tbc".to_string()); // todo: improve
        properties
    }
}