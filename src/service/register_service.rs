use autonomi::{Client, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::register::RegisterAddress;
use log::{info, warn};
use mockall_double::double;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[double]
use crate::client::RegisterCachingClient;
use crate::error::{CreateError, GetError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::StoreType;
use crate::error::register_error::RegisterError;
#[double]
use crate::service::resolver_service::ResolverService;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Register {
    pub name: Option<String>,
    pub content: String,
    #[schema(read_only)]
    pub address: Option<String>,
}

impl Register {
    pub fn new(name: Option<String>, content: String, address: Option<String>) -> Self {
        Register { name, content, address } 
    }
}

#[derive(Debug)]
pub struct RegisterService {
    register_caching_client: RegisterCachingClient,
    ant_tp_config: AntTpConfig,
    resolver_service: ResolverService,
}

impl RegisterService {

    pub fn new(register_caching_client: RegisterCachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        RegisterService { register_caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_register(&self, register: Register, evm_wallet: Wallet, store_type: StoreType) -> Result<Register, RegisterError> {
        match register.name {
            Some(name) => {
                let app_secret_key = self.ant_tp_config.get_app_private_key()?;
                let register_key = Client::register_key_from_name(&app_secret_key, name.as_str());

                info!("Create register from name [{}] and content [{}]", name, register.content);
                let content = Client::register_value_from_bytes(hex::decode(register.content.clone())?.as_slice())?;
                let register_address = self.register_caching_client
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
                let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
                if resolved_address.clone() != register_key.public_key().to_hex() {
                    warn!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name);
                    return Err(UpdateError::NotDerivedAddress(
                        format!("Address [{}] is not derived from name [{}].", resolved_address.clone(), name)).into());
                }

                info!("Update register with name [{}] and content [{}]", name, register.content);
                let content = Client::register_value_from_bytes(hex::decode(register.content.clone())?.as_slice())?;
                self.register_caching_client
                    .register_update(&register_key, content, PaymentOption::from(&evm_wallet), store_type)
                    .await?;
                info!("Queued command to update register with name [{}]", name);
                Ok(Register::new(Some(name), register.content, Some(resolved_address)))
            },
            None => Err(RegisterError::UpdateError(UpdateError::InvalidData("Name must be provided".to_string())))
        }
    }

    pub async fn get_register(&self, address: String) -> Result<Register, RegisterError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        match RegisterAddress::from_hex(resolved_address.as_str()) { // todo: create autonomi PR to change to ParseAddressError
            Ok(register_address) => {
                let content = self.register_caching_client.register_get(&register_address).await?;
                Ok(Register::new(None, hex::encode(content), Some(register_address.to_hex())))
            },
            Err(e) => Err(RegisterError::GetError(GetError::BadAddress(e.to_string()))),
        }
    }

    pub async fn get_register_history(&self, address: String) -> Result<Vec<Register>, RegisterError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        match RegisterAddress::from_hex(resolved_address.as_str()) { // todo: create autonomi PR to change to ParseAddressError
            Ok(register_address) => {
                let content_vec = self.register_caching_client.register_history(&register_address).await?.collect().await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use crate::client::MockRegisterCachingClient;
    use crate::service::resolver_service::MockResolverService;
    use crate::config::anttp_config::AntTpConfig;
    use clap::Parser;
    use autonomi::SecretKey;

    fn create_test_service(mock_client: MockRegisterCachingClient, mock_resolver: MockResolverService) -> RegisterService {
        let ant_tp_config = AntTpConfig::try_parse_from(&[
            "anttp",
            "--app-private-key",
            "0000000000000000000000000000000000000000000000000000000000000001"
        ]).unwrap();
        
        RegisterService::new(mock_client, ant_tp_config, mock_resolver)
    }

    #[tokio::test]
    async fn test_create_register_success() {
        let mut mock_client = MockRegisterCachingClient::default();
        let mock_resolver = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        
        let name = "test_register".to_string();
        let content_hex = hex::encode("test content");
        let register = Register::new(Some(name.clone()), content_hex.clone(), None);

        let app_secret_key = SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let register_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let expected_address = RegisterAddress::new(register_key.public_key());

        mock_client
            .expect_register_create()
            .with(eq(register_key), always(), always(), eq(StoreType::Network))
            .times(1)
            .returning(move |_, _, _, _| Ok(expected_address));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.create_register(register, evm_wallet, StoreType::Network).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.name, Some(name));
        assert_eq!(created.content, content_hex);
        assert_eq!(created.address, Some(expected_address.to_hex()));
    }

    #[tokio::test]
    async fn test_create_register_no_name_error() {
        let mock_client = MockRegisterCachingClient::default();
        let mock_resolver = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        
        let register = Register::new(None, "content".to_string(), None);

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.create_register(register, evm_wallet, StoreType::Network).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RegisterError::CreateError(CreateError::InvalidData(msg)) => assert_eq!(msg, "Name must be provided"),
            _ => panic!("Expected InvalidData error"),
        }
    }

    #[tokio::test]
    async fn test_update_register_success() {
        let mut mock_client = MockRegisterCachingClient::default();
        let mut mock_resolver = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        
        let name = "test_register".to_string();
        let content_hex = hex::encode("updated content");
        let app_secret_key = SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let register_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let address = register_key.public_key().to_hex();
        
        let register = Register::new(Some(name.clone()), content_hex.clone(), None);

        mock_resolver
            .expect_resolve_name()
            .with(eq(address.clone()))
            .times(1)
            .returning(move |addr| Some(addr.to_string()));

        mock_client
            .expect_register_update()
            .with(eq(register_key), always(), always(), eq(StoreType::Network))
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.update_register(address.clone(), register, evm_wallet, StoreType::Network).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.name, Some(name));
        assert_eq!(updated.address, Some(address));
    }

    #[tokio::test]
    async fn test_update_register_address_mismatch_error() {
        let mock_client = MockRegisterCachingClient::default();
        let mut mock_resolver = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        
        let name = "test_register".to_string();
        let wrong_address = "wrong_address".to_string();
        let register = Register::new(Some(name), "content".to_string(), None);

        mock_resolver
            .expect_resolve_name()
            .returning(move |addr| Some(addr.to_string()));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.update_register(wrong_address, register, evm_wallet, StoreType::Network).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RegisterError::UpdateError(UpdateError::NotDerivedAddress(_)) => (),
            _ => panic!("Expected NotDerivedAddress error"),
        }
    }

    #[tokio::test]
    async fn test_get_register_success() {
        let mut mock_client = MockRegisterCachingClient::default();
        let mut mock_resolver = MockResolverService::default();
        
        // I will use a different way to create RegisterAddress to see what's happening.
        let name = "some_name";
        let app_secret_key = SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let register_key = Client::register_key_from_name(&app_secret_key, name);
        let register_address = RegisterAddress::new(register_key.public_key());
        let address_hex = register_address.to_hex();

        mock_resolver
            .expect_resolve_name()
            .returning(move |addr| Some(addr.to_string()));

        mock_client
            .expect_register_get()
            .with(eq(register_address))
            .times(1)
            .returning(move |_| {
                Ok(unsafe { std::mem::transmute::<[u8; 32], autonomi::register::RegisterValue>([1u8; 32]) })
            });

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.get_register(address_hex.to_string()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_register_bad_address_error() {
        let mock_client = MockRegisterCachingClient::default();
        let mut mock_resolver = MockResolverService::default();
        
        let bad_address = "invalid".to_string();

        mock_resolver
            .expect_resolve_name()
            .returning(move |addr| Some(addr.to_string()));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.get_register(bad_address).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RegisterError::GetError(GetError::BadAddress(_)) => (),
            _ => panic!("Expected BadAddress error"),
        }
    }
}