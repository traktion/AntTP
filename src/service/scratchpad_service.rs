use autonomi::{Client, ScratchpadAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::StoreType;
use crate::error::{GetError, UpdateError};
use mockall_double::double;
#[double]
use crate::client::scratchpad_caching_client::ScratchpadCachingClient;
#[double]
use crate::service::resolver_service::ResolverService;
use crate::error::scratchpad_error::ScratchpadError;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Scratchpad {
    pub name: Option<String>,
    #[schema(read_only)]
    pub address: Option<String>,
    #[schema(read_only)]
    pub data_encoding: Option<u64>,
    #[schema(read_only)]
    pub signature: Option<String>,
    pub content: Option<String>,
    #[schema(read_only)]
    pub counter: Option<u64>,
}

impl Scratchpad {
    pub fn new(name: Option<String>, address: Option<String>, data_encoding: Option<u64>, signature: Option<String>, content: Option<String>, counter: Option<u64>) -> Self {
        Scratchpad { name, address, data_encoding, signature, content, counter }
    }
}

#[derive(Debug)]
pub struct ScratchpadService {
    scratchpad_caching_client: ScratchpadCachingClient,
    ant_tp_config: AntTpConfig,
    resolver_service: ResolverService,
}

impl ScratchpadService {
    pub fn new(scratchpad_caching_client: ScratchpadCachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        ScratchpadService { scratchpad_caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_scratchpad(&self, scratchpad: Scratchpad, evm_wallet: Wallet, is_encrypted: bool, store_type: StoreType) -> Result<Scratchpad, ScratchpadError> {
        let name = scratchpad.name.clone().ok_or_else(|| ScratchpadError::GetError(GetError::DerivationNameMissing("Name required to create scratchpad".to_string())))?;
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let content = scratchpad.content.clone().unwrap_or_else(|| "".to_ascii_lowercase());
        info!("Create scratchpad from name [{}] for data sized [{}]", name, content.len());
        let decoded_content = Bytes::from(BASE64_STANDARD.decode(content).unwrap_or_else(|_| Vec::new()));
        let scratchpad_address: autonomi::ScratchpadAddress = if is_encrypted {
            self.scratchpad_caching_client
                .scratchpad_create(
                    &scratchpad_key, 1, &decoded_content, PaymentOption::from(&evm_wallet), store_type)
                .await?
        } else {
            self.scratchpad_caching_client
                .scratchpad_create_public(
                    &scratchpad_key, 1, &decoded_content, PaymentOption::from(&evm_wallet), store_type)
                .await?
        };
        info!("Queued command to create{}scratchpad at [{}]", if !is_encrypted { "public " } else { "" }, scratchpad_address.to_hex());
        Ok(Scratchpad::new(
            Some(name.clone()), Some(scratchpad_address.to_hex()), None, None, scratchpad.content, None))
    }

    pub async fn update_scratchpad(&self, address: String, name: String, scratchpad: Scratchpad, evm_wallet: Wallet, is_encrypted: bool, store_type: StoreType) -> Result<Scratchpad, ScratchpadError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        if resolved_address.clone() != scratchpad_key.public_key().to_hex() {
            return Err(UpdateError::NotDerivedAddress(
                format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name)).into());
        }

        let content = scratchpad.content.clone().unwrap_or_else(|| "".to_ascii_lowercase());
        info!("Update {}scratchpad with name [{}] with data sized [{}]", if !is_encrypted { "public " } else { "" }, name, content.len());
        let decoded_content = BASE64_STANDARD.decode(content.clone()).unwrap_or_else(|_| Vec::new());
        if is_encrypted {
            self.scratchpad_caching_client
                .scratchpad_update(
                    &scratchpad_key, 1, &Bytes::from(decoded_content.clone()), store_type)
                .await?
        } else {
            self.scratchpad_caching_client
                .scratchpad_update_public(
                    &scratchpad_key, 1, &Bytes::from(decoded_content.clone()), PaymentOption::from(&evm_wallet), store_type)
                .await?
        };
        info!("Updated {}scratchpad with name [{}]", if !is_encrypted { "public " } else { "" }, name);
        Ok(Scratchpad::new(Some(name), Some(resolved_address), None, None, scratchpad.content, None))
    }

    pub async fn get_scratchpad(&self, address: String, name: Option<String>, is_encrypted: bool) -> Result<Scratchpad, ScratchpadError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        match ScratchpadAddress::from_hex(resolved_address.as_str()) {
            Ok(scratchpad_address) => {
                let scratchpad: autonomi::Scratchpad = self.scratchpad_caching_client.scratchpad_get(&scratchpad_address).await?;
                info!("Retrieved {}scratchpad at address [{}] with data sized [{}]", if !is_encrypted { "public " } else { "" }, resolved_address, scratchpad.encrypted_data().len());
                let content = self.get_scratchpad_content(&resolved_address, name, is_encrypted, &scratchpad)?;
                let signature = BASE64_STANDARD.encode(scratchpad.signature().to_bytes());
                Ok(Scratchpad::new(
                    None, Some(resolved_address), Some(scratchpad.data_encoding()), Some(signature), Some(content), Some(scratchpad.counter())))
            },
            Err(e) => Err(ScratchpadError::GetError(GetError::BadAddress(e.to_string())))
        }
    }

    pub fn get_scratchpad_content(&self, address: &String, name: Option<String>, is_encrypted: bool, scratchpad: &autonomi::Scratchpad) -> Result<String, ScratchpadError> {
        if is_encrypted {
            match name {
                Some(name) => {
                    match SecretKey::from_hex(self.ant_tp_config.app_private_key.as_str()) {
                        Ok(app_secret_key) => {
                            let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
                            if *address == scratchpad_key.public_key().to_hex() {
                                match scratchpad.decrypt_data(&scratchpad_key) {
                                    Ok(data) => Ok(BASE64_STANDARD.encode(data)),
                                    Err(e) => Err(ScratchpadError::GetError(GetError::Decryption(
                                        format!("Failed to decrypt private scratchpad at address [{}]: [{}]", address, e.to_string()))))
                                }
                            } else {
                                warn!("Address [{}] is not derived from name [{}].", address, name);
                                Err(ScratchpadError::GetError(GetError::NotDerivedAddress(
                                    format!("Address [{}] is not derived from name [{}].", address, name))))
                            }
                        },
                        Err(e) => Err(ScratchpadError::GetError(GetError::DerivationKeyMissing(
                            format!("AntTP app secret key must be provided: [{}]", e.to_string()))))
                    }
                },
                None => Err(ScratchpadError::GetError(GetError::DerivationNameMissing(
                    "Name required to get private scratchpad".to_string()))),
            }
        } else {
            Ok(BASE64_STANDARD.encode(scratchpad.encrypted_data()))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use crate::client::scratchpad_caching_client::MockScratchpadCachingClient;
    use crate::service::resolver_service::MockResolverService;

    fn create_test_service(mock_client: MockScratchpadCachingClient, mock_resolver: MockResolverService) -> ScratchpadService {
        use clap::Parser;
        let ant_tp_config = AntTpConfig::parse_from(&[
            "anttp",
            "--app-private-key",
            "0000000000000000000000000000000000000000000000000000000000000001"
        ]);

        ScratchpadService::new(mock_client, ant_tp_config, mock_resolver)
    }

    #[tokio::test]
    async fn test_create_scratchpad_success() {
        let mut mock_client = MockScratchpadCachingClient::default();
        let mock_resolver = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);

        let name = "test_scratchpad".to_string();
        let content = BASE64_STANDARD.encode("test content");
        let scratchpad = Scratchpad::new(Some(name.clone()), None, None, None, Some(content), None);

        let app_secret_key = autonomi::SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let expected_address = autonomi::ScratchpadAddress::new(scratchpad_key.public_key());

        mock_client
            .expect_scratchpad_create()
            .with(eq(scratchpad_key), eq(1), always(), always(), eq(StoreType::Network))
            .times(1)
            .returning(move |_, _, _, _, _| Ok(expected_address));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.create_scratchpad(scratchpad, evm_wallet, true, StoreType::Network).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().address.unwrap(), expected_address.to_hex());
    }

    #[tokio::test]
    async fn test_get_scratchpad_success() {
        let mut mock_client = MockScratchpadCachingClient::default();
        let mut mock_resolver = MockResolverService::default();

        let name = "test_scratchpad".to_string();
        let app_secret_key = autonomi::SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let address = autonomi::ScratchpadAddress::new(scratchpad_key.public_key());
        let address_hex = address.to_hex();

        mock_resolver
            .expect_resolve_name()
            .with(eq(address_hex.clone()))
            .times(1)
            .returning(move |addr| Some(addr.to_string()));

        let content = Bytes::from("test content");
        let scratchpad = autonomi::Scratchpad::new(&scratchpad_key, 1, &content, 0);

        mock_client
            .expect_scratchpad_get()
            .with(eq(address))
            .times(1)
            .returning(move |_| Ok(scratchpad.clone()));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.get_scratchpad(address_hex, Some(name), true).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content.unwrap(), BASE64_STANDARD.encode(content));
    }
}
