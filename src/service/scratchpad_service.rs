use autonomi::{Client, ScratchpadAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::ScratchpadCachingClient;
use crate::error::{GetError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::StoreType;
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
}

impl ScratchpadService {
    pub fn new(scratchpad_caching_client: ScratchpadCachingClient, ant_tp_config: AntTpConfig) -> Self {
        ScratchpadService { scratchpad_caching_client, ant_tp_config }
    }

    pub async fn create_scratchpad(&self, scratchpad: Scratchpad, evm_wallet: Wallet, is_encrypted: bool, store_type: StoreType) -> Result<Scratchpad, ScratchpadError> {
        let name = scratchpad.name.clone().ok_or_else(|| ScratchpadError::GetError(GetError::DerivationNameMissing("Name required to create scratchpad".to_string())))?;
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let content = scratchpad.content.clone().unwrap_or_else(|| "".to_ascii_lowercase());
        info!("Create scratchpad from name [{}] for data sized [{}]", name, content.len());
        let decoded_content = Bytes::from(BASE64_STANDARD.decode(content).unwrap_or_else(|_| Vec::new()));
        let scratchpad_address = if is_encrypted {
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
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        if address.clone() != scratchpad_key.public_key().to_hex() {
            return Err(UpdateError::NotDerivedAddress(
                format!("Address [{}] is not derived from name [{}].", address.clone(), name)).into());
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
        Ok(Scratchpad::new(Some(name), Some(address), None, None, scratchpad.content, None))
    }

    pub async fn get_scratchpad(&self, address: String, name: Option<String>, is_encrypted: bool) -> Result<Scratchpad, ScratchpadError> {
        match ScratchpadAddress::from_hex(address.as_str()) {
            Ok(scratchpad_address) => {
                let scratchpad = self.scratchpad_caching_client.scratchpad_get(&scratchpad_address).await?;
                info!("Retrieved {}scratchpad at address [{}] with data sized [{}]", if !is_encrypted { "public " } else { "" }, address, scratchpad.encrypted_data().len());
                let content = self.get_scratchpad_content(&address, name, is_encrypted, &scratchpad)?;
                let signature = BASE64_STANDARD.encode(scratchpad.signature().to_bytes());
                Ok(Scratchpad::new(
                    None, Some(address), Some(scratchpad.data_encoding()), Some(signature), Some(content), Some(scratchpad.counter())))
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
