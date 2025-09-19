use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorPreconditionFailed};
use autonomi::{ChunkAddress, Client, PointerAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::CacheType;
use crate::service::resolver_service::ResolverService;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Pointer {
    name: Option<String>,
    content: String,
    #[schema(read_only)]
    address: Option<String>,
    #[schema(read_only)]
    counter: Option<u64>,
    #[schema(read_only)]
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
    resolver_service: ResolverService,
}

impl PointerService {

    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        PointerService { caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_pointer(&self, pointer: Pointer, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let pointer_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());

        let chunk_address = ChunkAddress::from_hex(pointer.content.clone().as_str()).unwrap();
        info!("Create pointer from name [{}] for chunk [{}]", pointer.name.clone().unwrap(), chunk_address);
        match self.caching_client
            .pointer_create(&pointer_key, PointerTarget::ChunkAddress(chunk_address), PaymentOption::from(&evm_wallet), cache_only)
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

    pub async fn update_pointer(&self, address: String, pointer: Pointer, cache_only: Option<CacheType>) -> Result<HttpResponse, Error> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let pointer_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());
        if resolved_address.clone() != pointer_key.public_key().to_hex() {
            warn!("Address [{}] is not derived from name [{}].", resolved_address.clone(), pointer.name.clone().unwrap());
            return Err(ErrorPreconditionFailed(format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), pointer.name.clone().unwrap())));
        }

        let chunk_address = ChunkAddress::from_hex(pointer.content.clone().as_str()).unwrap();
        info!("Update pointer with name [{}] for chunk [{}]", pointer.name.clone().unwrap(), chunk_address);
        match self.caching_client
            .pointer_update(&pointer_key, PointerTarget::ChunkAddress(chunk_address), cache_only)
            .await {
            Ok(()) => {
                info!("Updated pointer with name [{}]", pointer.name.clone().unwrap());
                let response_pointer = Pointer::new(pointer.name, pointer.content, Some(resolved_address), None, None);
                Ok(HttpResponse::Ok().json(response_pointer))
            }
            Err(e) => {
                warn!("Failed to update pointer: [{:?}]", e);
                Err(ErrorInternalServerError("Failed to update pointer"))
            }
        }
    }

    pub async fn get_pointer(&self, address: String) -> Result<HttpResponse, Error> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);
        info!("Get pointer with resolved_address [{}]", resolved_address);
        let pointer_address = PointerAddress::from_hex(resolved_address.as_str()).expect("failed to create pointer from hex");
        match self.caching_client.pointer_get(&pointer_address).await {
            Ok(pointer) => {
                info!("Retrieved pointer at address [{}] value [{}]", resolved_address, pointer.target().to_hex());
                let response_pointer = Pointer::new(
                    None, pointer.target().to_hex(), Some(resolved_address), Some(pointer.counter()), None);
                Ok(HttpResponse::Ok().json(response_pointer).into())
            }
            Err(e) => {
                warn!("Failed to retrieve pointer at address [{}]: [{:?}]", resolved_address, e);
                Err(ErrorInternalServerError("Failed to retrieve pointer at address"))
            }
        }
    }
}