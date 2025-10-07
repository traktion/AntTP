use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{GraphEntry};
use autonomi::client::payment::PaymentOption;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::command::{Command, CommandError};

pub struct CreateGraphEntryCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    graph_entry: GraphEntry,
    payment_option: PaymentOption,
}

impl CreateGraphEntryCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, graph_entry: GraphEntry, payment_option: PaymentOption) -> Self {
        Self { client_harness, graph_entry, payment_option }
    }
}

#[async_trait]
impl Command for CreateGraphEntryCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let graph_entry_hex = self.graph_entry.address().to_string();
                debug!("creating graph entry at [{}] async", graph_entry_hex);
                match client.graph_entry_put(self.graph_entry.clone(), self.payment_option.clone()).await {
                    Ok(_) => {
                        info!("graph entry at address [{}] created successfully", graph_entry_hex);
                        Ok(())
                    },
                    Err(e) => Err(CommandError::from(e.to_string()))
                }
            },
            None => Err(CommandError::from(String::from("network offline"))),
        }
    }
}