use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::GraphEntry;
use autonomi::client::payment::PaymentOption;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct CreateGraphEntryCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    graph_entry: GraphEntry,
    payment_option: PaymentOption,
}

impl CreateGraphEntryCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, graph_entry: GraphEntry, payment_option: PaymentOption) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, graph_entry, payment_option }
    }
}

const STRUCT_NAME: &'static str = "CreateGraphEntryCommand";

#[async_trait]
impl Command for CreateGraphEntryCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client,
            None => return Err(CommandError::from(String::from("network offline")))
        };

        let graph_entry_hex = self.graph_entry.address().to_string();
        debug!("creating graph entry at [{}] async", graph_entry_hex);
        match client.graph_entry_put(self.graph_entry.clone(), self.payment_option.clone()).await {
            Ok(_) => {
                info!("graph entry at address [{}] created successfully", graph_entry_hex);
                Ok(())
            },
            Err(e) => Err(CommandError::from(e.to_string()))
        }
    }

    fn get_action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.graph_entry.owner.to_hex());
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
        properties.insert("graph_entry_owner".to_string(), self.graph_entry.owner.to_hex());
        properties
    }
}