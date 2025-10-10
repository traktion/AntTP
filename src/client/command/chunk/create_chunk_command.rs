use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::Chunk;
use autonomi::client::payment::PaymentOption;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::Command;
use crate::client::command::error::CommandError;

pub struct CreateChunkCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    chunk: Chunk,
    payment_option: PaymentOption,
}

impl CreateChunkCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, chunk: Chunk, payment_option: PaymentOption,) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, chunk, payment_option }
    }
}

const STRUCT_NAME: &'static str = "CreateChunkCommand";

#[async_trait]
impl Command for CreateChunkCommand {    
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        let chunk_address_hex = self.chunk.address.to_hex();
        debug!("creating chunk with address [{}] on network", chunk_address_hex);
        match client.chunk_put(&self.chunk, self.payment_option.clone()).await {
            Ok(_) => {
                info!("chunk at address [{}] created successfully", chunk_address_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::from(e.to_string()))
        }
    }

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.chunk.address().to_hex());
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
        properties.insert("chunk_address".to_string(), self.chunk.address().to_hex());
        properties
    }
}