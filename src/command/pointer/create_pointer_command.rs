use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{PointerAddress, SecretKey};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::command::{Command, CommandError};

pub struct CreatePointerCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    target: PointerTarget,
    payment_option: PaymentOption,
}

impl CreatePointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, target: PointerTarget,
               payment_option: PaymentOption,) -> Self {
        Self { client_harness, owner, target, payment_option }
    }
}

#[async_trait]
impl Command for CreatePointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let pointer_address_hex = PointerAddress::new(self.owner.public_key()).to_hex();
                debug!("creating pointer at [{}] async", pointer_address_hex);
                match client.pointer_create(&self.owner, self.target.clone(), self.payment_option.clone()).await {
                    Ok(_) => {
                        info!("pointer at address [{}] created successfully", pointer_address_hex);
                        Ok(())
                    },
                    Err(e) => Err(CommandError::from(e.to_string()))
                }
            },
            None => Err(CommandError::from(String::from("network offline"))),
        }
    }
}