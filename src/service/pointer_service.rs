use autonomi::{ChunkAddress, Client, PointerAddress, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use log::{info, warn};
use mockall_double::double;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[double]
use crate::client::pointer_caching_client::PointerCachingClient;
use crate::error::{CreateError, UpdateError};
use crate::config::anttp_config::AntTpConfig;
use crate::controller::{StoreType, DataKey};
use crate::error::pointer_error::PointerError;
#[double]
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

#[cfg(test)]
mod tests {
    use crate::client::pointer_caching_client::MockPointerCachingClient;
    use crate::service::resolver_service::MockResolverService;
    use autonomi::{PublicKey};
    use autonomi::client::payment::PaymentOption;
    use clap::Parser;
    use mockall::predicate::*;
    use super::*;

    fn create_test_service(
        mock_pointer_caching_client: MockPointerCachingClient,
        mock_resolver_service: MockResolverService,
    ) -> PointerService {
        let config = AntTpConfig::try_parse_from(&[
            "anttp",
            "--resolver-private-key",
            "55dcbc4624699d219b8ec293339a3b81e68815397f5a502026784d8122d09fce",
            "--app-private-key",
            "55dcbc4624699d219b8ec293339a3b81e68815397f5a502026784d8122d09fce",
        ]).unwrap();
        PointerService::new(mock_pointer_caching_client, config, mock_resolver_service)
    }

    #[tokio::test]
    async fn test_create_pointer_success() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_resolver_service = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        let name = "test_pointer".to_string();
        let content = "a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527a73a699f9f96a80045391a23356ed0e3".to_string();
        let pointer = super::Pointer {
            name: Some(name.clone()),
            content: content.clone(),
            address: None,
            counter: None,
            cost: None,
        };

        mock_resolver_service
            .expect_resolve_name()
            .returning(|_| None);

        mock_resolver_service
            .expect_is_immutable_address()
            .returning(|_| true);

        let pointer_address = PointerAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527a73a699f9f96a80045391a23356ed0e3").unwrap();

        mock_pointer_caching_client
            .expect_pointer_create()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(pointer_address.clone()));

        let service = create_test_service(mock_pointer_caching_client, mock_resolver_service);
        let result = service.create_pointer(pointer, evm_wallet, StoreType::Network, DataKey::Personal).await;

        match result {
            Ok(created_pointer) => {
                assert_eq!(created_pointer.name, Some(name));
                assert_eq!(created_pointer.content, content);
                assert!(created_pointer.address.is_some());
            },
            Err(e) => panic!("Error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_pointer_no_name_error() {
        let mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_resolver_service = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        let pointer = super::Pointer {
            name: None,
            content: "some_content".to_string(),
            address: None,
            counter: None,
            cost: None,
        };

        let service = create_test_service(mock_pointer_caching_client, mock_resolver_service);
        let result = service.create_pointer(pointer, evm_wallet, StoreType::Network, DataKey::Personal).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_pointer_success() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_resolver_service = MockResolverService::default();
        let name = "test_pointer".to_string();
        let content = "a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527a73a699f9f96a80045391a23356ed0e3".to_string();
        
        let config = AntTpConfig::try_parse_from(&[
            "anttp",
            "--resolver-private-key",
            "55dcbc4624699d219b8ec293339a3b81e68815397f5a502026784d8122d09fce",
        ]).unwrap();
        let secret_key = config.get_resolver_private_key().unwrap();
        let pointer_key = autonomi::Client::register_key_from_name(&secret_key, name.as_str());
        let address = pointer_key.public_key().to_hex();

        let pointer = super::Pointer {
            name: Some(name.clone()),
            content: content.clone(),
            address: None,
            counter: None,
            cost: None,
        };

        let address_val = address.clone();
        mock_resolver_service
            .expect_resolve_name()
            .with(eq(address.clone()))
            .times(1)
            .returning(move |_| Some(address_val.clone()));
        
        mock_resolver_service
            .expect_is_immutable_address()
            .returning(|_| true);

        mock_pointer_caching_client
            .expect_pointer_update()
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let service = PointerService::new(mock_pointer_caching_client, config, mock_resolver_service);
        let result = service.update_pointer(address.clone(), pointer, StoreType::Network, DataKey::Resolver).await;

        match result {
            Ok(updated_pointer) => {
                assert_eq!(updated_pointer.name, Some(name));
                assert_eq!(updated_pointer.address, Some(address));
            },
            Err(e) => panic!("Update Error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_update_pointer_address_mismatch_error() {
        let mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_resolver_service = MockResolverService::default();
        let name = "test_pointer".to_string();
        let address = "wrong_address".to_string();
        let pointer = super::Pointer {
            name: Some(name.clone()),
            content: "content".to_string(),
            address: None,
            counter: None,
            cost: None,
        };

        mock_resolver_service
            .expect_resolve_name()
            .returning(|_| None);

        let service = create_test_service(mock_pointer_caching_client, mock_resolver_service);
        let result = service.update_pointer(address, pointer, StoreType::Network, DataKey::Personal).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_pointer_success() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_resolver_service = MockResolverService::default();
        let address = "a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527a73a699f9f96a80045391a23356ed0e3".to_string();
        let target_hex = "b40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527a73a699f9f96a80045391a23356ed0e3";
        let target = PointerTarget::ChunkAddress(ChunkAddress::from_hex(target_hex).unwrap());

        let address_val = address.clone();
        mock_resolver_service
            .expect_resolve_name()
            .with(eq(address.clone()))
            .times(1)
            .returning(move |_| Some(address_val.clone()));

        let owner_sk = SecretKey::random();
        let autonomi_pointer = autonomi::Pointer::new(&owner_sk, 1, target);

        mock_pointer_caching_client
            .expect_pointer_get()
            .times(1)
            .returning(move |_| Ok(autonomi_pointer.clone()));

        let service = create_test_service(mock_pointer_caching_client, mock_resolver_service);
        let result = service.get_pointer(address.clone()).await;

        match result {
            Ok(retrieved_pointer) => {
                assert_eq!(retrieved_pointer.content, target_hex);
                assert_eq!(retrieved_pointer.address, Some(address));
                assert_eq!(retrieved_pointer.counter, Some(1));
            },
            Err(e) => panic!("Get Error: {:?}", e),
        }
    }
}