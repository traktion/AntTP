use actix_web::web::Data;
use async_trait::async_trait;
use autonomi::{PointerAddress, SecretKey};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::client::command::error::CommandError;
use crate::client::command::Command;

pub struct CreatePointerCommand {
    id: u128,
    client_harness: Data<Mutex<ClientHarness>>,
    owner: SecretKey,
    target: PointerTarget,
    counter: Option<u64>,
    payment_option: PaymentOption,
}

impl CreatePointerCommand {
    pub fn new(client_harness: Data<Mutex<ClientHarness>>, owner: SecretKey, target: PointerTarget,
               counter: Option<u64>, payment_option: PaymentOption,) -> Self {
        let id = rand::random::<u128>();
        Self { id, client_harness, owner, target, counter, payment_option }
    }
}

const STRUCT_NAME: &'static str = "CreatePointerCommand";

#[async_trait]
impl Command for CreatePointerCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let client = self.client_harness.get_ref().lock().await.get_client().await?;
        let pointer_address_hex = PointerAddress::new(self.owner.public_key()).to_hex();
        debug!("creating pointer at [{}] async", pointer_address_hex);
        client.pointer_create(&self.owner, self.target.clone(), self.payment_option.clone()).await?;

        if let Some(counter) = self.counter {
            debug!("updating pointer at [{}] to counter [{}]", pointer_address_hex, counter);
            // decrement counter, as pointer_update_from will increment
            let pointer = autonomi::Pointer::new(&self.owner, counter - 1, self.target.clone());
            client.pointer_update_from(&pointer, &self.owner, self.target.clone()).await?;
        }

        info!("pointer at address [{}] created successfully", pointer_address_hex);
        Ok(())
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.owner.to_hex());
        hasher.update(self.target.to_hex());
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
        properties.insert("owner".to_string(), self.owner.to_hex());
        properties.insert("target".to_string(), self.target.to_hex());
        if let Some(counter) = self.counter {
            properties.insert("counter".to_string(), counter.to_string());
        }
        properties
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::anttp_config::AntTpConfig;
    use clap::Parser;
    use ant_evm::EvmNetwork;
    use autonomi::ChunkAddress;

    fn create_test_command(counter: Option<u64>) -> CreatePointerCommand {
        let config = AntTpConfig::try_parse_from(&[
            "anttp",
            "--app-private-key",
            "55dcbc4624699d219b8ec293339a3b81e68815397f5a502026784d8122d09fce",
        ]).unwrap();
        let client_harness = ClientHarness::new(EvmNetwork::ArbitrumOne, config.clone());
        let owner = config.get_app_private_key().unwrap();
        let target = PointerTarget::ChunkAddress(ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap());
        let payment_option = PaymentOption::Wallet(autonomi::Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne));

        CreatePointerCommand::new(
            Data::new(Mutex::new(client_harness)),
            owner,
            target,
            counter,
            payment_option,
        )
    }

    #[test]
    fn test_create_pointer_command_new() {
        let counter = Some(10);
        let command = create_test_command(counter);
        assert_eq!(command.counter, counter);
        assert_eq!(command.name(), "CreatePointerCommand");
    }

    #[test]
    fn test_create_pointer_command_properties() {
        let counter = Some(10);
        let command = create_test_command(counter);
        let properties = command.properties();
        assert_eq!(properties.get("counter").unwrap(), "10");
        assert!(properties.contains_key("owner"));
        assert!(properties.contains_key("target"));
    }

    #[test]
    fn test_create_pointer_command_properties_no_counter() {
        let command = create_test_command(None);
        let properties = command.properties();
        assert!(!properties.contains_key("counter"));
    }
}