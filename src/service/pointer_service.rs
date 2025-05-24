use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorPreconditionFailed};
use ant_evm::{AttoTokens};
use autonomi::{ChunkAddress, Client, PointerAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use crate::anttp_config::AntTpConfig;

#[derive(Serialize, Deserialize)]
pub struct Pointer {
    name: Option<String>,
    target: String,
    address: Option<String>,
    cost: Option<AttoTokens>,
    counter: Option<u32>
}

impl Pointer {
    pub fn new(name: Option<String>, target: String, address: Option<String>, cost: Option<AttoTokens>, counter: Option<u32>) -> Self {
        Pointer { name, target, address, cost, counter } 
    }
}

pub struct PointerService {
    autonomi_client: Client,
    ant_tp_config: AntTpConfig,
}

// todo: create/update different PointerTarget types
impl PointerService {

    pub fn new(autonomi_client: Client, ant_tp_config: AntTpConfig) -> Self {
        PointerService { autonomi_client, ant_tp_config }
    }

    pub async fn create_pointer(&self, pointer: Pointer, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let pointer_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());

        let chunk_address = ChunkAddress::from_hex(pointer.target.clone().as_str()).unwrap();
        info!("Create pointer from name [{}] for chunk [{}]", pointer.name.clone().unwrap(), chunk_address);
        match self.autonomi_client
            .pointer_create(&pointer_key, PointerTarget::ChunkAddress(chunk_address), PaymentOption::from(&evm_wallet))
            .await {
                Ok((cost, pointer_address)) => {
                    info!("Created pointer at [{}] for [{}] attos", pointer_address.to_hex(), cost);
                    let response_pointer = Pointer::new(pointer.name, pointer.target, Some(pointer_address.to_hex()), Some(cost), None);
                    Ok(HttpResponse::Created().json(response_pointer))
                }
                Err(e) => {
                    // todo: refine error handling to return appropriate messages / payloads
                    warn!("Failed to create pointer: [{:?}]", e);
                    Err(ErrorInternalServerError("Failed to create pointer:"))
                }
        }
    }

    pub async fn update_pointer(&self, address: String, pointer: Pointer) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let pointer_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());
        if address.clone() != pointer_key.public_key().to_hex() {
            warn!("Address [{}] is not derived from name [{}].", address.clone(), pointer.name.clone().unwrap());
            return Err(ErrorPreconditionFailed(format!("Address [{}] is not derived from name [{}].", address.clone(), pointer.name.clone().unwrap())));
        }

        let chunk_address = ChunkAddress::from_hex(pointer.target.clone().as_str()).unwrap();
        info!("Update pointer with name [{}] for chunk [{}]", pointer.name.clone().unwrap(), chunk_address);
        match self.autonomi_client
            .pointer_update(&pointer_key, PointerTarget::ChunkAddress(chunk_address))
            .await {
            Ok(()) => {
                info!("Updated pointer with name [{}]", pointer.name.clone().unwrap());
                let response_pointer = Pointer::new(pointer.name, pointer.target, Some(address), None, None);
                Ok(HttpResponse::Ok().json(response_pointer))
            }
            Err(e) => {
                warn!("Failed to update pointer: [{:?}]", e);
                Err(ErrorInternalServerError("Failed to update pointer"))
            }
        }
    }

    pub async fn get_pointer(&self, address: String) -> Result<HttpResponse, Error> {
        let pointer_address = PointerAddress::from_hex(address.as_str()).unwrap();
        match self.autonomi_client.pointer_get(&pointer_address).await {
            Ok(pointer) => {
                info!("Retrieved pointer at address [{}] value [{}]", address, pointer.target().to_hex());
                let response_pointer = Pointer::new(
                    None, pointer.target().to_hex(), Some(address), None, Some(pointer.counter()));
                Ok(HttpResponse::Ok().json(response_pointer).into())
            }
            Err(e) => {
                warn!("Failed to retrieve register at address [{}]: [{:?}]", address, e);
                Err(ErrorInternalServerError("Failed to retrieve register at address"))
            }
        }
    }
}