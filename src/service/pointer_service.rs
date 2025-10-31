use autonomi::{ChunkAddress, Client, PointerAddress, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::error::{GetError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::CacheType;
use crate::error::pointer_error::PointerError;
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

    pub async fn create_pointer(&self, pointer: Pointer, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Pointer, PointerError> {
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let pointer_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());

        let chunk_address = ChunkAddress::from_hex(pointer.content.clone().as_str()).unwrap();
        info!("Create pointer from name [{}] for chunk [{}]", pointer.name.clone().unwrap(), chunk_address);
        let pointer_address = self.caching_client
            .pointer_create(&pointer_key, PointerTarget::ChunkAddress(chunk_address), PaymentOption::from(&evm_wallet), cache_only)
            .await?;
        info!("Queued command to create pointer at [{}]", pointer_address.to_hex());
        Ok(Pointer::new(pointer.name, pointer.content, Some(pointer_address.to_hex()), None, None))
    }

    pub async fn update_pointer(&self, address: String, pointer: Pointer, cache_only: Option<CacheType>) -> Result<Pointer, PointerError> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let pointer_key = Client::register_key_from_name(&app_secret_key, pointer.name.clone().unwrap().as_str());
        if resolved_address.clone() != pointer_key.public_key().to_hex() {
            warn!("Address [{}] is not derived from name [{}].", resolved_address.clone(), pointer.name.clone().unwrap());
            return Err(UpdateError::NotDerivedAddress(
                format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), pointer.name.clone().unwrap())).into());
        }

        let chunk_address = ChunkAddress::from_hex(pointer.content.clone().as_str()).unwrap();
        info!("Update pointer with name [{}] for chunk [{}]", pointer.name.clone().unwrap(), chunk_address);
        self.caching_client.pointer_update(&pointer_key, PointerTarget::ChunkAddress(chunk_address), cache_only).await?;
        info!("Updated pointer with name [{}]", pointer.name.clone().unwrap());
        Ok(Pointer::new(pointer.name, pointer.content, Some(resolved_address), None, None))
    }

    pub async fn get_pointer(&self, address: String) -> Result<Pointer, PointerError> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);

        info!("Get pointer with resolved_address [{}]", resolved_address);
        match PointerAddress::from_hex(resolved_address.as_str()) {
            Ok(pointer_address) => match self.caching_client.pointer_get(&pointer_address).await {
                Ok(pointer) => {
                    info!("Retrieved pointer at address [{}] value [{}]", resolved_address, pointer.target().to_hex());
                    Ok(Pointer::new(None, pointer.target().to_hex(), Some(resolved_address), Some(pointer.counter()), None))
                }
                Err(e) => {
                    warn!("Failed to retrieve pointer at address [{}]: [{:?}]", resolved_address, e);
                    Err(e)
                }
            },
            Err(e) => Err(PointerError::GetError(GetError::BadAddress(e.to_string()))),
        }
    }
}