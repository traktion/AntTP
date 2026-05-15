use actix_web::web::Data;
use ant_core::data::DataChunk;
use async_trait::async_trait;
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
    chunk: DataChunk,
}

impl CreateChunkCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, chunk: DataChunk) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, chunk }
    }
}

const STRUCT_NAME: &'static str = "CreateChunkCommand";

#[async_trait]
impl Command for CreateChunkCommand {    
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let chunk = &self.chunk.clone();
        let chunk_address_hex = hex::encode(&chunk.address);
        debug!("creating chunk with address [{}] on network", chunk_address_hex);
        client.chunk_put(chunk.content.clone()).await?;
        info!("chunk at address [{}] created successfully", chunk_address_hex);
        Ok(())
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(hex::encode(self.chunk.address));
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
        properties.insert("chunk_address".to_string(), hex::encode(self.chunk.address));
        properties
    }
}