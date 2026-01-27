use autonomi::{ChunkAddress, Wallet};
use autonomi::client::chunk as autonomi_chunk;
use autonomi::client::payment::PaymentOption;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use log::{info, warn};
use mockall_double::double;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[double]
use crate::client::chunk_caching_client::ChunkCachingClient;
use crate::error::{CreateError, GetError};
use crate::error::chunk_error::ChunkError;
use crate::controller::StoreType;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Chunk {
    pub content: Option<String>,
    #[schema(read_only)]
    pub address: Option<String>,
}

impl Chunk {
    pub fn new(content: Option<String>, address: Option<String>) -> Self {
        Chunk { content, address }
    }
}

#[derive(Debug)]
pub struct ChunkService {
    chunk_caching_client: ChunkCachingClient
}

impl ChunkService {

    pub fn new(chunk_caching_client: ChunkCachingClient) -> Self {
        ChunkService { chunk_caching_client }
    }

    pub async fn create_chunk_binary(&self, bytes: Bytes, evm_wallet: Wallet, store_type: StoreType) -> Result<Chunk, ChunkError> {
        let chunk_data =  autonomi_chunk::Chunk::new(bytes);
        self.create_chunk_raw(chunk_data, evm_wallet, store_type).await
    }

    pub async fn create_chunk(&self, chunk: Chunk, evm_wallet: Wallet, store_type: StoreType) -> Result<Chunk, ChunkError> {
        let content = match chunk.content.clone() {
            Some(content) => content,
            None => return Err(ChunkError::CreateError(CreateError::InvalidData("Empty chunk payload".to_string())))
        };
        let decoded_content = BASE64_STANDARD.decode(content).unwrap_or_else(|_| Vec::new());
        let chunk_data =  autonomi_chunk::Chunk::new(Bytes::from(decoded_content.clone()));

        self.create_chunk_raw(chunk_data, evm_wallet, store_type).await
    }

    pub async fn create_chunk_raw(&self, chunk: autonomi_chunk::Chunk, evm_wallet: Wallet, store_type: StoreType) -> Result<Chunk, ChunkError> {
        info!("Create chunk at address [{}]", chunk.address.to_hex());
        let chunk_address = self.chunk_caching_client.chunk_put(&chunk, PaymentOption::from(&evm_wallet), store_type).await?;
        info!("Queued command to create chunk at [{}]", chunk_address.to_hex());
        Ok(Chunk::new(None, Some(chunk_address.to_hex())))
    }

    pub async fn get_chunk_binary(&self, address: String) -> Result<autonomi::Chunk, ChunkError> {
        match ChunkAddress::from_hex(address.as_str()) {
            Ok(chunk_address) => match self.chunk_caching_client.chunk_get_internal(&chunk_address).await {
                Ok(chunk) => {
                    info!("Retrieved chunk at address [{}]", address);
                    Ok(chunk)
                }
                Err(e) => {
                    warn!("Failed to retrieve chunk at address [{}]: [{:?}]", address, e);
                    Err(ChunkError::GetError(GetError::RecordNotFound(e.to_string())))
                }
            },
            Err(e) => Err(ChunkError::GetError(GetError::BadAddress(e.to_string())))
        }
    }

    pub async fn get_chunk(&self, address: String) -> Result<Chunk, ChunkError> {
        match ChunkAddress::from_hex(address.as_str()) {
            Ok(chunk_address) => match self.chunk_caching_client.chunk_get_internal(&chunk_address).await {
                Ok(chunk) => {
                    info!("Retrieved chunk at address [{}]", address);
                    Ok(Chunk::new(Some(BASE64_STANDARD.encode(chunk.value)), Some(address)))
                }
                Err(e) => {
                    warn!("Failed to retrieve chunk at address [{}]: [{:?}]", address, e);
                    Err(ChunkError::GetError(GetError::RecordNotFound(e.to_string())))
                }
            },
            Err(e) => Err(ChunkError::GetError(GetError::BadAddress(e.to_string()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use crate::client::chunk_caching_client::MockChunkCachingClient;
    use autonomi::Network;

    fn create_test_service(mock_client: MockChunkCachingClient) -> ChunkService {
        ChunkService::new(mock_client)
    }

    #[tokio::test]
    async fn test_create_chunk_binary_success() {
        let mut mock_client = MockChunkCachingClient::default();
        let bytes = Bytes::from("test content");
        let wallet = Wallet::new_with_random_wallet(Network::ArbitrumOne);
        let store_type = StoreType::Network;

        mock_client
            .expect_chunk_put()
            .with(always(), always(), eq(store_type.clone()))
            .times(1)
            .returning(|_, _, _| Ok(ChunkAddress::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap()));

        let service = create_test_service(mock_client);
        let result = service.create_chunk_binary(bytes, wallet, store_type).await;

        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.address, Some("0000000000000000000000000000000000000000000000000000000000000000".to_string()));
    }

    #[tokio::test]
    async fn test_create_chunk_success() {
        let mut mock_client = MockChunkCachingClient::default();
        let content = BASE64_STANDARD.encode("test content");
        let chunk_input = Chunk::new(Some(content), None);
        let wallet = Wallet::new_with_random_wallet(Network::ArbitrumOne);
        let store_type = StoreType::Network;

        mock_client
            .expect_chunk_put()
            .with(always(), always(), eq(store_type.clone()))
            .times(1)
            .returning(|_, _, _| Ok(ChunkAddress::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap()));

        let service = create_test_service(mock_client);
        let result = service.create_chunk(chunk_input, wallet, store_type).await;

        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.address, Some("0000000000000000000000000000000000000000000000000000000000000000".to_string()));
    }

    #[tokio::test]
    async fn test_create_chunk_empty_payload_error() {
        let mock_client = MockChunkCachingClient::default();
        let chunk_input = Chunk::new(None, None);
        let wallet = Wallet::new_with_random_wallet(Network::ArbitrumOne);
        let store_type = StoreType::Network;

        let service = create_test_service(mock_client);
        let result = service.create_chunk(chunk_input, wallet, store_type).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ChunkError::CreateError(CreateError::InvalidData(msg)) => assert_eq!(msg, "Empty chunk payload"),
            _ => panic!("Expected InvalidData error"),
        }
    }

    #[tokio::test]
    async fn test_get_chunk_binary_success() {
        let mut mock_client = MockChunkCachingClient::default();
        let address_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let chunk_address = ChunkAddress::from_hex(address_hex).unwrap();
        let expected_bytes = Bytes::from("test content");

        mock_client
            .expect_chunk_get_internal()
            .with(eq(chunk_address))
            .times(1)
            .returning(move |_| Ok(autonomi::Chunk::new(expected_bytes.clone())));

        let service = create_test_service(mock_client);
        let result = service.get_chunk_binary(address_hex.to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, Bytes::from("test content"));
    }

    #[tokio::test]
    async fn test_get_chunk_binary_not_found_error() {
        let mut mock_client = MockChunkCachingClient::default();
        let address_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let chunk_address = ChunkAddress::from_hex(address_hex).unwrap();

        mock_client
            .expect_chunk_get_internal()
            .with(eq(chunk_address))
            .times(1)
            .returning(|_| Err(ChunkError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let service = create_test_service(mock_client);
        let result = service.get_chunk_binary(address_hex.to_string()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ChunkError::GetError(GetError::RecordNotFound(_)) => (),
            _ => panic!("Expected RecordNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_chunk_success() {
        let mut mock_client = MockChunkCachingClient::default();
        let address_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let chunk_address = ChunkAddress::from_hex(address_hex).unwrap();
        let expected_bytes = Bytes::from("test content");

        mock_client
            .expect_chunk_get_internal()
            .with(eq(chunk_address))
            .times(1)
            .returning(move |_| Ok(autonomi::Chunk::new(expected_bytes.clone())));

        let service = create_test_service(mock_client);
        let result = service.get_chunk(address_hex.to_string()).await;

        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.content, Some(BASE64_STANDARD.encode("test content")));
        assert_eq!(chunk.address, Some(address_hex.to_string()));
    }

    #[tokio::test]
    async fn test_get_chunk_bad_address_error() {
        let mock_client = MockChunkCachingClient::default();
        let address_hex = "invalid_address";

        let service = create_test_service(mock_client);
        let result = service.get_chunk(address_hex.to_string()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ChunkError::GetError(GetError::BadAddress(_)) => (),
            _ => panic!("Expected BadAddress error"),
        }
    }
}