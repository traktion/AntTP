use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorPreconditionFailed};
use ant_evm::{AttoTokens};
use autonomi::{Client, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::register::{RegisterAddress};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use crate::anttp_config::AntTpConfig;

#[derive(Serialize, Deserialize)]
pub struct Register {
    name: Option<String>,
    content: String,
    address: Option<String>,
    cost: Option<AttoTokens>,
}

impl Register {
    pub fn new(name: Option<String>, content: String, address: Option<String>, cost: Option<AttoTokens>) -> Self {
        Register { name, content, address, cost } 
    }
}

pub struct RegisterService {
    autonomi_client: Client,
    ant_tp_config: AntTpConfig,
}

impl RegisterService {

    pub fn new(autonomi_client: Client, ant_tp_config: AntTpConfig) -> Self {
        RegisterService { autonomi_client, ant_tp_config }
    }

    pub async fn create_register(&self, register: Register, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let register_key = Client::register_key_from_name(&app_secret_key, register.name.clone().unwrap().as_str());

        info!("Create register from name [{}] and content [{}]", register.name.clone().unwrap(), register.content);
        let content = Client::register_value_from_bytes(hex::decode(register.content.clone()).unwrap().as_slice()).unwrap();
        match self.autonomi_client
            .register_create(&register_key, content, PaymentOption::from(&evm_wallet))
            .await {
                Ok((cost, register_address)) => {
                    info!("Created register at [{}] for [{}] attos", register_address.to_hex(), cost);
                    let response_register = Register::new(
                        register.name, register.content, Some(register_address.to_hex()), Some(cost));
                    Ok(HttpResponse::Created().json(response_register))
                }
                Err(e) => {
                    // todo: refine error handling to return appropriate messages / payloads
                    warn!("Failed to create register: [{:?}]", e);
                    Err(ErrorInternalServerError("Failed to create register:"))
                }
        }
    }

    pub async fn update_register(&self, address: String, register: Register, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let register_key = Client::register_key_from_name(&app_secret_key, register.name.clone().unwrap().as_str());
        if address.clone() != register_key.public_key().to_hex() {
            warn!("Address [{}] is not derived from name [{}].", address.clone(), register.name.clone().unwrap());
            return Err(ErrorPreconditionFailed(format!("Address [{}] is not derived from name [{}].", address.clone(), register.name.clone().unwrap())));
        }

        info!("Update register with name [{}] and content [{}]", register.name.clone().unwrap(), register.content);
        let content = Client::register_value_from_bytes(hex::decode(register.content.clone()).unwrap().as_slice()).unwrap();
        match self.autonomi_client
            .register_update(&register_key, content, PaymentOption::from(&evm_wallet))
            .await {
            Ok(cost) => {
                info!("Updated register with name [{}] for [{}] attos", register.name.clone().unwrap(), cost);
                let response_register = Register::new(Some(register.name.unwrap()), register.content, Some(address), Some(cost));
                Ok(HttpResponse::Ok().json(response_register))
            }
            Err(e) => {
                warn!("Failed to update register: [{:?}]", e);
                Err(ErrorInternalServerError("Failed to update register"))
            }
        }
    }

    pub async fn get_register(&self, address: String) -> Result<HttpResponse, Error> {
        let register_address = RegisterAddress::from_hex(address.as_str()).unwrap();
        match self.autonomi_client.register_get(&register_address).await {
            Ok(content) => {
                info!("Retrieved register at address [{}] value [{}]", register_address, hex::encode(content));
                let response_register = Register::new(
                    None, hex::encode(content), Some(register_address.to_hex()), None);
                Ok(HttpResponse::Ok().json(response_register).into())
            }
            Err(e) => {
                warn!("Failed to retrieve register at address [{}]: [{:?}]", register_address.to_hex(), e);
                Err(ErrorInternalServerError("Failed to retrieve register at address"))
            }
        }
    }

    pub async fn get_register_history(&self, address: String) -> Result<HttpResponse, Error> {
        let register_address = RegisterAddress::from_hex(address.as_str()).unwrap();
        match self.autonomi_client.register_history(&register_address).collect().await {
            Ok(content_vec) => {
                let content_flattened: String = content_vec.iter().map(|&c|hex::encode(c)).collect();
                info!("Retrieved register history [{}] at address [{}]", content_flattened, register_address);
                let mut response_registers = Vec::new();
                content_vec.iter().for_each(|content|response_registers.push(
                    Register::new(None, hex::encode(content), Some(register_address.to_hex()), None)
                ));
                Ok(HttpResponse::Ok().json(response_registers).into())
            }
            Err(e) => {
                warn!("Failed to retrieve register history at address [{}]: [{:?}]", register_address.to_hex(), e);
                Err(ErrorInternalServerError("Failed to retrieve register history at address"))
            }
        }
    }
}