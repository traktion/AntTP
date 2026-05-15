use actix_web::web::Data;
use async_trait::async_trait;
use bytes::Bytes;
use hex::ToHex;
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
}

impl CreatePublicDataCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, data: Bytes) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, data }
    }
}

const STRUCT_NAME: &'static str = "CreatePublicDataCommand";

#[async_trait]
impl Command for CreatePublicDataCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let result = client.data_upload(self.data.clone()).await?;
        let data_address = result.data_map.infos().get(0).unwrap().dst_hash;
        info!("chunk at address [{}] created successfully", data_address.encode_hex::<String>());
        Ok(())
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
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
        properties.insert("data".to_string(), "tbc".to_string()); // todo: improve
        properties
    }
}