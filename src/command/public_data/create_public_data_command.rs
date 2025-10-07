use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use log::info;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::command::{Command, CommandError};

pub struct CreatePublicDataCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    data: Bytes,
    payment_option: PaymentOption,
}

impl CreatePublicDataCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, data: Bytes,
               payment_option: PaymentOption) -> Self {
        Self { client_harness, data, payment_option }
    }
}

#[async_trait]
impl Command for CreatePublicDataCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                match client.data_put_public(self.data.clone(), self.payment_option.clone()).await {
                    Ok((_, data_address)) => {
                        info!("chunk at address [{}] created successfully", data_address);
                        Ok(())
                    },
                    Err(e) => Err(CommandError::from(e.to_string()))
                }
            },
            None => Err(CommandError::from(String::from("network offline"))),
        }
    }
}