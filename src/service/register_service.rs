use autonomi::{Client, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::register::RegisterAddress;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::error::{GetError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::CacheType;
use crate::error::register_error::RegisterError;
use crate::service::resolver_service::ResolverService;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Register {
    name: Option<String>,
    content: String,
    #[schema(read_only)]
    address: Option<String>,
    #[schema(read_only)]
    cost: Option<String>,
}

impl Register {
    pub fn new(name: Option<String>, content: String, address: Option<String>, cost: Option<String>) -> Self {
        Register { name, content, address, cost } 
    }
}

pub struct RegisterService {
    caching_client: CachingClient,
    ant_tp_config: AntTpConfig,
    resolver_service: ResolverService,
}

impl RegisterService {

    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        RegisterService { caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_register(&self, register: Register, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Register, RegisterError> {
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let register_key = Client::register_key_from_name(&app_secret_key, register.name.clone().unwrap().as_str());

        info!("Create register from name [{}] and content [{}]", register.name.clone().unwrap(), register.content);
        let content = Client::register_value_from_bytes(hex::decode(register.content.clone()).expect("failed to decode hex").as_slice()).unwrap();
        let (cost, register_address) = self.caching_client
            .register_create(&register_key, content, PaymentOption::from(&evm_wallet), cache_only)
            .await?;
        info!("Created register at [{}] for [{}] attos", register_address.to_hex(), cost);
        Ok(Register::new(register.name, register.content, Some(register_address.to_hex()), Some(cost.to_string())))
    }

    pub async fn update_register(&self, address: String, register: Register, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Register, RegisterError> {
        let app_secret_key = self.ant_tp_config.get_app_private_key()?;
        let register_key = Client::register_key_from_name(&app_secret_key, register.name.clone().unwrap().as_str());
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);
        if resolved_address.clone() != register_key.public_key().to_hex() {
            warn!("Address [{}] is not derived from name [{}].", resolved_address.clone(), register.name.clone().unwrap());
            return Err(UpdateError::NotDerivedAddress(
                format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), register.name.clone().unwrap())).into());
        }

        info!("Update register with name [{}] and content [{}]", register.name.clone().unwrap(), register.content);
        let content = Client::register_value_from_bytes(hex::decode(register.content.clone()).expect("failed to decode hex").as_slice()).unwrap();
        let cost = self.caching_client
            .register_update(&register_key, content, PaymentOption::from(&evm_wallet), cache_only)
            .await?;
        info!("Updated register with name [{}] for [{}] attos", register.name.clone().unwrap(), cost);
        Ok(Register::new(Some(register.name.unwrap()), register.content, Some(resolved_address), Some(cost.to_string())))
    }

    pub async fn get_register(&self, address: String) -> Result<Register, RegisterError> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);
        match RegisterAddress::from_hex(resolved_address.as_str()) {
            Ok(register_address) => match self.caching_client.register_get(&register_address).await {
                Ok(content) => {
                    info!("Retrieved register at address [{}] value [{}]", register_address, hex::encode(content));
                    Ok(Register::new(None, hex::encode(content), Some(register_address.to_hex()), None))
                }
                Err(e) => {
                    warn!("Failed to retrieve register at address [{}]: [{:?}]", register_address.to_hex(), e);
                    Err(e)
                }
            },
            Err(e) => Err(RegisterError::GetError(GetError::BadAddress(e.to_string()))),
        }
    }

    pub async fn get_register_history(&self, address: String) -> Result<Vec<Register>, RegisterError> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).unwrap_or(address);
        let register_address = RegisterAddress::from_hex(resolved_address.as_str()).unwrap();
        match self.caching_client.register_history(&register_address).await?.collect().await {
            Ok(content_vec) => {
                let content_flattened: String = content_vec.iter().map(|&c|hex::encode(c)).collect();
                info!("Retrieved register history [{}] at address [{}]", content_flattened, register_address);
                let mut response_registers = Vec::new();
                content_vec.iter().for_each(|content|response_registers.push(
                    Register::new(None, hex::encode(content), Some(register_address.to_hex()), None)
                ));
                Ok(response_registers)
            }
            Err(e) => Err(RegisterError::GetError(GetError::RecordNotFound(e.to_string())))
        }
    }
}