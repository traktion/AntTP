use autonomi::{Client, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::register::RegisterAddress;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::error::{CreateError, GetError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::StoreType;
use crate::error::register_error::RegisterError;
use crate::service::resolver_service::ResolverService;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Register {
    name: Option<String>,
    content: String,
    #[schema(read_only)]
    address: Option<String>,
}

impl Register {
    pub fn new(name: Option<String>, content: String, address: Option<String>) -> Self {
        Register { name, content, address } 
    }
}

#[derive(Debug)]
pub struct RegisterService {
    caching_client: CachingClient,
    ant_tp_config: AntTpConfig,
    resolver_service: ResolverService,
}

impl RegisterService {

    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        RegisterService { caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_register(&self, register: Register, evm_wallet: Wallet, store_type: StoreType) -> Result<Register, RegisterError> {
        match register.name {
            Some(name) => {
                let app_secret_key = self.ant_tp_config.get_app_private_key()?;
                let register_key = Client::register_key_from_name(&app_secret_key, name.as_str());

                info!("Create register from name [{}] and content [{}]", name, register.content);
                let content = Client::register_value_from_bytes(hex::decode(register.content.clone())?.as_slice())?;
                let register_address = self.caching_client
                    .register_create(&register_key, content, PaymentOption::from(&evm_wallet), store_type)
                    .await?;
                info!("Queued command to create register at [{}]", register_address.to_hex());
                Ok(Register::new(Some(name), register.content, Some(register_address.to_hex())))
            },
            None => Err(RegisterError::CreateError(CreateError::InvalidData("Name must be provided".to_string())))
        }
    }

    pub async fn update_register(&self, address: String, register: Register, evm_wallet: Wallet, store_type: StoreType) -> Result<Register, RegisterError> {
        match register.name {
            Some(name) => {
                let app_secret_key = self.ant_tp_config.get_app_private_key()?;
                let register_key = Client::register_key_from_name(&app_secret_key, name.as_str());
                let resolved_address = self.resolver_service.resolve_bookmark(&address).await.unwrap_or(address);
                if resolved_address.clone() != register_key.public_key().to_hex() {
                    warn!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name);
                    return Err(UpdateError::NotDerivedAddress(
                        format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name)).into());
                }

                info!("Update register with name [{}] and content [{}]", name, register.content);
                let content = Client::register_value_from_bytes(hex::decode(register.content.clone())?.as_slice())?;
                self.caching_client
                    .register_update(&register_key, content, PaymentOption::from(&evm_wallet), store_type)
                    .await?;
                info!("Queued command to update register with name [{}]", name);
                Ok(Register::new(Some(name), register.content, Some(resolved_address)))
            },
            None => Err(RegisterError::UpdateError(UpdateError::InvalidData("Name must be provided".to_string())))
        }
    }

    pub async fn get_register(&self, address: String) -> Result<Register, RegisterError> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).await.unwrap_or(address);
        match RegisterAddress::from_hex(resolved_address.as_str()) { // todo: create autonomi PR to change to ParseAddressError
            Ok(register_address) => {
                let content = self.caching_client.register_get(&register_address).await?;
                Ok(Register::new(None, hex::encode(content), Some(register_address.to_hex())))
            },
            Err(e) => Err(RegisterError::GetError(GetError::BadAddress(e.to_string()))),
        }
    }

    pub async fn get_register_history(&self, address: String) -> Result<Vec<Register>, RegisterError> {
        let resolved_address = self.resolver_service.resolve_bookmark(&address).await.unwrap_or(address);
        match RegisterAddress::from_hex(resolved_address.as_str()) { // todo: create autonomi PR to change to ParseAddressError
            Ok(register_address) => {
                let content_vec = self.caching_client.register_history(&register_address).await?.collect().await?;
                let content_flattened: String = content_vec.iter().map(|&c| hex::encode(c)).collect();
                info!("Retrieved register history [{}] at address [{}]", content_flattened, register_address);
                let mut response_registers = Vec::new();
                content_vec.iter().for_each(|content| response_registers.push(
                    Register::new(None, hex::encode(content), Some(register_address.to_hex()))
                ));
                Ok(response_registers)
            },
            Err(e) => Err(RegisterError::GetError(GetError::BadAddress(e.to_string()))),
        }
    }
}