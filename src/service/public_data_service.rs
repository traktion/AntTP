use ant_core::data::XorName;
use bytes::Bytes;
use hex::{FromHex, ToHex};
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
use mockall::mock;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicData {
    #[schema(read_only)]
    address: Option<String>,
}

#[derive(Clone)]
pub struct PublicDataService {
    public_data_caching_client: PublicDataCachingClient,
    resolver_service: ResolverService
}

mock! {
    pub PublicDataService {
        pub fn new(public_data_caching_client: PublicDataCachingClient, resolver_service: ResolverService) -> Self;
        pub async fn create_public_data(&self, bytes: Bytes, store_type: StoreType) -> Result<Chunk, PublicDataError>;
        pub async fn push_public_data(&self, address: String, store_type: StoreType) -> Result<Chunk, PublicDataError>;
        pub async fn get_public_data_binary(&self, address: String) -> Result<Bytes, PublicDataError>;
    }
    impl Clone for PublicDataService {
        fn clone(&self) -> Self;
    }
}

impl PublicDataService {
    pub fn new(public_data_caching_client: PublicDataCachingClient, resolver_service: ResolverService) -> Self {
        Self { public_data_caching_client, resolver_service }
    }

    pub async fn create_public_data(&self, bytes: Bytes, store_type: StoreType) -> Result<Chunk, PublicDataError> {
        let xor_name: XorName = self.public_data_caching_client.data_put_public(bytes, store_type).await?;
        let xor_name_hex: String = xor_name.encode_hex();
        info!("Queued command to create public data at [{}]", xor_name_hex);
        Ok(Chunk::new(None, Some(xor_name_hex)))
    }

    pub async fn push_public_data(&self, address: String, store_type: StoreType) -> Result<Chunk, PublicDataError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let data_address = match XorName::from_hex(resolved_address.as_str()) {
            Ok(data_address) => data_address,
            Err(e) => return Err(PublicDataError::GetError(GetError::BadAddress(e.to_string())))
        };
        // Retrieve the public data (from cache or network)
        let bytes = self.public_data_caching_client.data_get_public(&data_address).await?;
        // Push to the target store type (memory/disk/network)
        let new_data_address = self.public_data_caching_client.data_put_public(bytes, store_type).await?;
        Ok(Chunk::new(None, Some(new_data_address.encode_hex())))
    }

    pub async fn get_public_data_binary(&self, address: String) -> Result<Bytes, PublicDataError> {
        let resolved_address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        match XorName::from_hex(resolved_address.as_str()) {
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

    fn create_test_service(mock_client: MockPublicDataCachingClient) -> PublicDataService {
        let mut mock_resolver = MockResolverService::default();
        mock_resolver.expect_resolve_name()
            .returning(|address| Some(address.clone()));
        PublicDataService::new(mock_client, mock_resolver)
    }

    #[tokio::test]
    async fn test_create_public_data_success() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let xor_name = XorName::default();
        let expected_hex = xor_name.encode_hex();

        mock_client
            .expect_data_put_public()
            .returning(move |_, _| Ok(xor_name));

        let service = create_test_service(mock_client);
        let bytes = Bytes::from("test data");

        let result = service.create_public_data(bytes, StoreType::Memory).await;
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.address, Some(expected_hex));
    }

    #[tokio::test]
    async fn test_push_public_data_success() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let xor_name = XorName::default();
        let expected_hex = xor_name.encode_hex();
        let bytes = Bytes::from("test data");

        let get_bytes = bytes.clone();
        mock_client
            .expect_data_get_public()
            .returning(move |_| Ok(get_bytes.clone()));

        mock_client
            .expect_data_put_public()
            .returning(move |_, _| Ok(xor_name));

        let service = create_test_service(mock_client);

        let result = service.push_public_data(xor_name.encode_hex(), StoreType::Network).await;
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.address, Some(expected_hex));
    }

    #[tokio::test]
    async fn test_get_public_data_binary_success() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let xor_name = XorName::default();
        let expected_hex = xor_name.encode_hex();
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