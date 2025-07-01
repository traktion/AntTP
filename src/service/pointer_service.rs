use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorPreconditionFailed};
use autonomi::{ChunkAddress, Client, PointerAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Pointer {
    name: Option<String>,
    content: String,
    address: Option<String>,
    counter: Option<u64>,
    cost: Option<String>,
}

impl Pointer {
    pub fn new(name: Option<String>, content: String, address: Option<String>, counter: Option<u64>, cost: Option<String>) -> Self {
        Pointer { name, content, address, counter, cost } 
    }
}

pub struct PointerService {
    caching_client: CachingClient,
    ant_tp_config: AntTpConfig,
}

impl PointerService {

    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig) -> Self {
        PointerService { caching_client, ant_tp_config }
    }

    pub async fn create_pointer(&self, pointer: Pointer, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let owner_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());

        let app_secret_key2 = SecretKey::from_hex("646dd9cae2c6ec140b9c5527084e8db2a04d7e68dc35a832bc1d1e020cfd45be").unwrap();
        let address_key = Client::register_key_from_name(&app_secret_key2, pointer.name.clone().unwrap().as_str());
        
        let chunk_address = ChunkAddress::from_hex(pointer.content.clone().as_str()).unwrap();
        info!("Create pointer from name [{}] for target [{}] at address [{}]", pointer.name.clone().unwrap(), chunk_address, address_key.public_key().to_hex());
        match self.caching_client
            .pointer_create(&owner_key, address_key.public_key(), PointerTarget::ChunkAddress(chunk_address), PaymentOption::from(&evm_wallet))
            .await {
                Ok((cost, pointer_address)) => {
                    info!("Created pointer at [{}] for [{}] attos", pointer_address.to_hex(), cost);
                    let response_pointer = Pointer::new(pointer.name, pointer.content, Some(pointer_address.to_hex()), None, Some(cost.to_string()));
                    Ok(HttpResponse::Created().json(response_pointer))
                }
                Err(e) => {
                    // todo: refine error handling to return appropriate messages / payloads
                    warn!("Failed to create pointer: [{:?}]", e);
                    Err(ErrorInternalServerError("Failed to create pointer"))
                }
        }
    }

    pub async fn update_pointer(&self, name: String, pointer: Pointer) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let owner_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());

        let app_secret_key2 = SecretKey::from_hex("646dd9cae2c6ec140b9c5527084e8db2a04d7e68dc35a832bc1d1e020cfd45be").unwrap();
        let address_key = Client::register_key_from_name(&app_secret_key2, name.clone().as_str());
        //let new_address_key = Client::register_key_from_name(&app_secret_key2, pointer.name.clone().unwrap().as_str());

        let chunk_address = ChunkAddress::from_hex(pointer.content.clone().as_str()).unwrap();
        info!("Update pointer with name [{}] for target [{}] at address [{}]", name.clone(), chunk_address, address_key.public_key().to_hex());
        match self.caching_client
            .pointer_update(&owner_key, address_key.public_key(), PointerTarget::ChunkAddress(chunk_address))
            .await {
            Ok(()) => {
                info!("Updated pointer with name [{}]", pointer.name.clone().unwrap());
                let response_pointer = Pointer::new(pointer.name, pointer.content, Some(address_key.public_key().to_hex()), None, None);
                Ok(HttpResponse::Ok().json(response_pointer))
            }
            Err(e) => {
                warn!("Failed to update pointer: [{:?}]", e);
                Err(ErrorInternalServerError("Failed to update pointer"))
            }
        }
    }

    pub async fn get_pointer(&self, name: String) -> Result<HttpResponse, Error> {
        let app_secret_key2 = SecretKey::from_hex("646dd9cae2c6ec140b9c5527084e8db2a04d7e68dc35a832bc1d1e020cfd45be").unwrap();
        let address_key = Client::register_key_from_name(&app_secret_key2, name.clone().as_str());
        
        let pointer_address = PointerAddress::from_hex(address_key.public_key().to_hex().as_str()).unwrap();
        match self.caching_client.pointer_get(&pointer_address).await {
            Ok(pointer) => {
                info!("Retrieved pointer with name [{}] at address [{}] with value [{}]", name, pointer_address.to_hex(), pointer.target().to_hex());
                let response_pointer = Pointer::new(
                    None, pointer.target().to_hex(), Some(address_key.public_key().to_hex()), Some(pointer.counter()), None);
                Ok(HttpResponse::Ok().json(response_pointer).into())
            }
            Err(e) => {
                warn!("Failed to retrieve pointer with name [{}] at address [{}]: [{:?}]", name, pointer_address.to_hex(), e);
                Err(ErrorInternalServerError("Failed to retrieve pointer at address"))
            }
        }
    }
}