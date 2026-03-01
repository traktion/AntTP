use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{info};
use mockall_double::double;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[double]
use crate::client::PublicDataCachingClient;
use crate::error::GetError;
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;
#[double]
use crate::service::resolver_service::ResolverService;
use crate::service::chunk_service::Chunk;
use mockall::automock;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicData {
    #[schema(read_only)]
    address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PublicDataService {
    public_data_caching_client: PublicDataCachingClient,
    resolver_service: ResolverService
}

#[automock]
impl PublicDataService {
    pub fn new(public_data_caching_client: PublicDataCachingClient, resolver_service: ResolverService) -> Self {
        Self { public_data_caching_client, resolver_service }
    }

    pub async fn create_public_data(&self, bytes: Bytes, evm_wallet: Wallet, store_type: StoreType) -> Result<Chunk, PublicDataError> {
        let data_address: DataAddress = self.public_data_caching_client.data_put_public(bytes, PaymentOption::from(&evm_wallet), store_type).await?;
        info!("Queued command to create public data at [{}]", data_address.to_hex());
        Ok(Chunk::new(None, Some(data_address.to_hex())))
    }

    pub async fn push_public_data(&self, address: String, evm_wallet: Wallet, store_type: StoreType) -> Result<Chunk, PublicDataError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let data_address = match DataAddress::from_hex(resolved_address.as_str()) {
            Ok(data_address) => data_address,
            Err(e) => return Err(PublicDataError::GetError(GetError::BadAddress(e.to_string())))
        };
        // Retrieve the public data (from cache or network)
        let bytes = self.public_data_caching_client.data_get_public(&data_address).await?;
        // Push to the target store type (memory/disk/network)
        let new_data_address = self.public_data_caching_client.data_put_public(bytes, PaymentOption::from(&evm_wallet), store_type).await?;
        Ok(Chunk::new(None, Some(new_data_address.to_hex())))
    }

    pub async fn get_public_data_binary(&self, address: String) -> Result<Bytes, PublicDataError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        match DataAddress::from_hex(resolved_address.as_str()) {
            Ok(data_address) => self.public_data_caching_client.data_get_public(&data_address).await,
            Err(e) => Err(PublicDataError::GetError(GetError::BadAddress(e.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::MockPublicDataCachingClient;
    use crate::service::resolver_service::MockResolverService;
    use autonomi::data::DataAddress;
    use autonomi::Wallet;
    use xor_name::XorName;

    fn create_test_service(mock_client: MockPublicDataCachingClient) -> PublicDataService {
        let mut mock_resolver = MockResolverService::default();
        mock_resolver.expect_resolve_name()
            .returning(|address| Some(address.clone()));
        PublicDataService::new(mock_client, mock_resolver)
    }

    #[tokio::test]
    async fn test_create_public_data_success() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let xor_name = XorName::from_content(b"test");
        let data_address = DataAddress::new(xor_name);
        let expected_hex = data_address.to_hex();

        mock_client
            .expect_data_put_public()
            .returning(move |_, _, _| Ok(data_address));

        let service = create_test_service(mock_client);
        let bytes = Bytes::from("test data");
        let wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);

        let result = service.create_public_data(bytes, wallet, StoreType::Memory).await;
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.address, Some(expected_hex));
    }

    #[tokio::test]
    async fn test_push_public_data_success() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let xor_name = XorName::from_content(b"test");
        let data_address = DataAddress::new(xor_name);
        let expected_hex = data_address.to_hex();
        let bytes = Bytes::from("test data");

        let get_bytes = bytes.clone();
        mock_client
            .expect_data_get_public()
            .returning(move |_| Ok(get_bytes.clone()));

        mock_client
            .expect_data_put_public()
            .returning(move |_, _, _| Ok(data_address));

        let service = create_test_service(mock_client);
        let wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);

        let result = service.push_public_data(expected_hex.clone(), wallet, StoreType::Network).await;
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.address, Some(expected_hex));
    }

    #[tokio::test]
    async fn test_get_public_data_binary_success() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let xor_name = XorName::from_content(b"test");
        let data_address = DataAddress::new(xor_name);
        let expected_hex = data_address.to_hex();
        let bytes = Bytes::from("test data");

        let get_bytes = bytes.clone();
        mock_client
            .expect_data_get_public()
            .returning(move |_| Ok(get_bytes.clone()));

        let service = create_test_service(mock_client);
        let result = service.get_public_data_binary(expected_hex).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), bytes);
    }
}