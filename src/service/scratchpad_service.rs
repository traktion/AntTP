use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorPreconditionFailed};
use autonomi::{Client, ScratchpadAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::config::anttp_config::AntTpConfig;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Scratchpad {
    pub name: Option<String>,
    pub address: Option<String>,
    pub data_encoding: Option<u64>,
    pub signature: Option<bool>,
    pub content: Option<String>,
    pub counter: Option<u64>,
    pub cost: Option<String>,
}

impl Scratchpad {
    pub fn new(name: Option<String>, address: Option<String>, data_encoding: Option<u64>, signature: Option<bool>, content: Option<String>, counter: Option<u64>, cost: Option<String>) -> Self {
        Scratchpad { name, address, data_encoding, signature, content, counter, cost }
    }
}

pub struct ScratchpadService {
    caching_client: CachingClient,
    ant_tp_config: AntTpConfig,
}

impl ScratchpadService {

    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig) -> Self {
        ScratchpadService { caching_client, ant_tp_config }
    }

    pub async fn create_scratchpad(&self, scratchpad: Scratchpad, evm_wallet: Wallet, is_encrypted: bool) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, scratchpad.name.clone().unwrap().as_str());

        let content = scratchpad.content.clone().unwrap_or_else(|| "".to_ascii_lowercase());
        info!("Create scratchpad from name [{}] for data sized [{}]", scratchpad.name.clone().unwrap(), content.len());
        let decoded_content = match BASE64_STANDARD.decode(content.clone()) {
            Ok(b) => b,
            Err(_) => Vec::new(),
        };
        let result = if is_encrypted {
            self.caching_client
                .scratchpad_create(&scratchpad_key, 1, &Bytes::from(decoded_content), PaymentOption::from(&evm_wallet))
                .await
        } else {
            self.caching_client
                .scratchpad_create_public(&scratchpad_key, 1, &Bytes::from(decoded_content), PaymentOption::from(&evm_wallet))
                .await
        };
        match result {
            Ok((cost, scratchpad_address)) => {
                info!("Created {}scratchpad at [{}] for [{}] attos", if !is_encrypted { "public " } else { "" }, scratchpad_address.to_hex(), cost);
                let response_scratchpad = Scratchpad::new(scratchpad.name, Some(scratchpad_address.to_hex()), None, None, scratchpad.content, None, Some(cost.to_string()));
                Ok(HttpResponse::Created().json(response_scratchpad))
            }
            Err(e) => {
                // todo: refine error handling to return appropriate messages / payloads
                warn!("Failed to create {}scratchpad: [{:?}]", if !is_encrypted { "public " } else { "" }, e);
                Err(ErrorInternalServerError(format!("Failed to create {}scratchpad", if !is_encrypted { "public " } else { "" })))
            }
        }
    }

    pub async fn update_scratchpad(&self, address: String, scratchpad: Scratchpad, evm_wallet: Wallet, is_encrypted: bool) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let scratchpad_key = Client::register_key_from_name(&app_secret_key, scratchpad.name.clone().unwrap().as_str());
        if address.clone() != scratchpad_key.public_key().to_hex() {
            warn!("Address [{}] is not derived from name [{}].", address.clone(), scratchpad.name.clone().unwrap());
            return Err(ErrorPreconditionFailed(format!("Address [{}] is not derived from name [{}].", address.clone(), scratchpad.name.clone().unwrap())));
        }

        let content = scratchpad.content.clone().unwrap_or_else(|| "".to_ascii_lowercase());
        info!("Update {}scratchpad with name [{}] with data sized [{}]", if !is_encrypted { "public " } else { "" }, scratchpad.name.clone().unwrap(), content.len());
        let decoded_content = BASE64_STANDARD.decode(content.clone()).unwrap_or_else(|_| Vec::new());
        let result = if is_encrypted {
            self.caching_client.scratchpad_update(&scratchpad_key, 1, &Bytes::from(decoded_content.clone())).await
        } else {
            self.caching_client.scratchpad_update_public(&scratchpad_key, 1, &Bytes::from(decoded_content.clone()), PaymentOption::from(&evm_wallet), scratchpad.counter.unwrap()).await
        };
        match result {
            Ok(()) => {
                info!("Updated {}scratchpad with name [{}]", if !is_encrypted { "public " } else { "" }, scratchpad.name.clone().unwrap());
                let response_scratchpad = Scratchpad::new(scratchpad.name, Some(address), None, None, scratchpad.content, None, None);
                Ok(HttpResponse::Ok().json(response_scratchpad))
            }
            Err(e) => {
                warn!("Failed to update {}scratchpad: [{:?}]", if !is_encrypted { "public " } else { "" }, e);
                Err(ErrorInternalServerError(format!("Failed to update {}scratchpad", if !is_encrypted { "public " } else { "" })))
            }
        }
    }

    pub async fn get_scratchpad(&self, address: String, name: Option<String>, is_encrypted: bool) -> Result<HttpResponse, Error> {
        let scratchpad_address = ScratchpadAddress::from_hex(address.as_str()).unwrap();
        match self.caching_client.scratchpad_get(&scratchpad_address).await {
            Ok(scratchpad) => {
                info!("Retrieved {}scratchpad at address [{}] with data sized [{}]", if !is_encrypted { "public " } else { "" }, address, scratchpad.encrypted_data().len());

                let content = if is_encrypted {
                    let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
                    let scratchpad_key = Client::register_key_from_name(&app_secret_key, name.clone().unwrap().as_str());
                    if address.clone() != scratchpad_key.public_key().to_hex() {
                        warn!("Address [{}] is not derived from name [{}].", address.clone(), name.clone().unwrap());
                        return Err(ErrorPreconditionFailed(format!("Address [{}] is not derived from name [{}].", address.clone(), name.clone().unwrap())));
                    }
                    
                    match scratchpad.decrypt_data(&scratchpad_key) {
                        Ok(data) => BASE64_STANDARD.encode(data),
                        Err(_) => "".to_string()
                    }
                } else {
                    BASE64_STANDARD.encode(scratchpad.encrypted_data())
                };
                let response_scratchpad = Scratchpad::new(None, Some(address), Some(scratchpad.data_encoding()), Some(scratchpad.verify_signature()), Some(content), Some(scratchpad.counter()), None);
                Ok(HttpResponse::Ok().json(response_scratchpad).into())
            }
            Err(e) => {
                warn!("Failed to retrieve {}scratchpad at address [{}]: [{:?}]", if !is_encrypted { "public " } else { "" }, address, e);
                Err(ErrorInternalServerError(format!("Failed to retrieve {}scratchpad at address", if !is_encrypted { "public " } else { "" })))
            }
        }
    }
}