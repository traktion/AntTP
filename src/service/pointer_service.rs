use autonomi::{ChunkAddress, Client, PointerAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::PointerCachingClient;
use crate::error::{CreateError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::{StoreType, DataKey};
use crate::error::pointer_error::PointerError;
use crate::service::resolver_service::ResolverService;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Pointer {
    pub name: Option<String>,
    pub content: String,
    #[schema(read_only)]
    pub address: Option<String>,
    pub counter: Option<u64>,
    #[schema(read_only)]
    pub cost: Option<String>,
}

impl Pointer {
    pub fn new(name: Option<String>, content: String, address: Option<String>, counter: Option<u64>, cost: Option<String>) -> Self {
        Pointer { name, content, address, counter, cost } 
    }
}

#[derive(Debug, Clone)]
pub struct PointerService {
    pointer_caching_client: PointerCachingClient,
    ant_tp_config: AntTpConfig,
    resolver_service: ResolverService,
}

impl PointerService {

    pub fn new(pointer_caching_client: PointerCachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        PointerService { pointer_caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_pointer(&self, pointer: Pointer, evm_wallet: Wallet, store_type: StoreType, data_key: DataKey) -> Result<Pointer, PointerError> {
        match pointer.name {
            Some(name) => {
                let secret_key = self.get_data_key(data_key)?;
                let pointer_key = Client::register_key_from_name(&secret_key, name.as_str());

                let pointer_target = self.get_pointer_target(&pointer.content)?;
                info!("Create pointer from name [{}] for chunk [{}]", name, &pointer.content);
                let pointer_address = self.pointer_caching_client
                    .pointer_create(&pointer_key, pointer_target, pointer.counter, PaymentOption::from(&evm_wallet), store_type)
                    .await?;
                info!("Queued command to create pointer at [{}]", pointer_address.to_hex());
                Ok(Pointer::new(Some(name), pointer.content, Some(pointer_address.to_hex()), None, None))
            },
            None => Err(PointerError::CreateError(CreateError::InvalidData("Name must be provided".to_string())))
        }
    }

    pub async fn update_pointer(&self, address: String, pointer: Pointer, store_type: StoreType, data_key: DataKey) -> Result<Pointer, PointerError> {
        match pointer.name {
            Some(name) => {
                let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
                let secret_key = self.get_data_key(data_key)?;
                let pointer_key = Client::register_key_from_name(&secret_key, name.as_str());
                if resolved_address.clone() != pointer_key.public_key().to_hex() {
                    warn!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name);
                    return Err(UpdateError::NotDerivedAddress(
                        format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name)).into());
                }

                let pointer_target = self.get_pointer_target(&pointer.content)?;
                info!("Update pointer with name [{}] for chunk [{}]", name, &pointer.content);
                self.pointer_caching_client.pointer_update(&pointer_key, pointer_target, pointer.counter, store_type).await?;
                info!("Updated pointer with name [{}]", name);
                Ok(Pointer::new(Some(name), pointer.content, Some(resolved_address), None, None))
            },
            None => Err(PointerError::CreateError(CreateError::InvalidData("Name must be provided".to_string())))
        }
    }

    pub fn get_resolver_address(&self, name: &String) -> Result<String, CreateError> {
        let secret_key = self.get_data_key(DataKey::Resolver)?;
        Ok(Client::register_key_from_name(&secret_key, name.as_str()).public_key().to_hex())
    }

    fn get_data_key(&self, data_key: DataKey) -> Result<SecretKey, CreateError> {
        match data_key {
            DataKey::Resolver => self.ant_tp_config.get_resolver_private_key(),
            DataKey::Personal => self.ant_tp_config.get_app_private_key(),
            DataKey::Custom(key) => match SecretKey::from_hex(&key.as_str()) {
                Ok(secret_key) => Ok(secret_key),
                Err(e) => Err(CreateError::DataKeyMissing(e.to_string()))
            }
        }
    }

    fn get_pointer_target(&self, content: &String) -> Result<PointerTarget, PointerError> {
        Ok(if self.resolver_service.is_immutable_address(&content) {
            PointerTarget::ChunkAddress(ChunkAddress::from_hex(content.clone().as_str())?)
        } else {
            PointerTarget::PointerAddress(PointerAddress::from_hex(content.clone().as_str())?)
        })
    }

    pub async fn get_pointer(&self, address: String) -> Result<Pointer, PointerError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);

        info!("Get pointer with resolved_address [{}]", resolved_address);
        let pointer_address = PointerAddress::from_hex(resolved_address.as_str())?;
        let pointer = self.pointer_caching_client.pointer_get(&pointer_address).await?;
        info!("Retrieved pointer at address [{}] value [{}]", resolved_address, pointer.target().to_hex());
        Ok(Pointer::new(None, pointer.target().to_hex(), Some(resolved_address), Some(pointer.counter()), None))
    }
}