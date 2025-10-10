use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use indexmap::IndexMap;
use log::info;
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct CreatePublicDataCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    data: Bytes,
    payment_option: PaymentOption,
}

impl CreatePublicDataCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, data: Bytes, payment_option: PaymentOption) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, data, payment_option }
    }
}

const STRUCT_NAME: &'static str = "CreatePublicDataCommand";

#[async_trait]
impl Command for CreatePublicDataCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        match client.data_put_public(self.data.clone(), self.payment_option.clone()).await {
            Ok((_, data_address)) => {
                info!("chunk at address [{}] created successfully", data_address);
                Ok(())
            },
            Err(e) => Err(CommandError::from(e.to_string()))
        }
    }

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
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
        properties.insert("data".to_string(), "tbc".to_string()); // todo: improve
        properties
    }
}