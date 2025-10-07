use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::SecretKey;
use autonomi::client::payment::PaymentOption;
use autonomi::register::{RegisterAddress, RegisterValue};
use log::{debug, info};
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::command::{Command, CommandError};

pub struct UpdateRegisterCommand {
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    register_value: RegisterValue,
    payment_option: PaymentOption,
}

impl UpdateRegisterCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, register_value: RegisterValue,
               payment_option: PaymentOption,) -> Self {
        Self { client_harness, owner, register_value, payment_option }
    }
}

#[async_trait]
impl Command for UpdateRegisterCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let register_address_hex = RegisterAddress::new(self.owner.public_key()).to_hex();
                debug!("updating register at [{}] async", register_address_hex);
                match client.register_update(&self.owner, self.register_value, self.payment_option.clone()).await {
                    Ok(_) => {
                        info!("register at address [{}] updated successfully", register_address_hex);
                        Ok(())
                    },
                    Err(e) => Err(CommandError::from(e.to_string()))
                }
            },
            None => Err(CommandError::from(String::from("network offline"))),
        }
    }
}