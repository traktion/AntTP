use autonomi::{ChunkAddress, Wallet};
use autonomi::client::chunk as autonomi_chunk;
use autonomi::client::payment::PaymentOption;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::error::{CreateError, GetError};
use crate::error::chunk_error::ChunkError;
use crate::controller::CacheType;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Chunk {
    content: Option<String>,
    #[schema(read_only)]
    address: Option<String>,
}

impl Chunk {
    pub fn new(content: Option<String>, address: Option<String>) -> Self {
        Chunk { content, address }
    }
}

pub struct ChunkService {
    caching_client: CachingClient
}

impl ChunkService {

    pub fn new(caching_client: CachingClient) -> Self {
        ChunkService { caching_client }
    }

    pub async fn create_chunk_binary(&self, bytes: Bytes, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Chunk, ChunkError> {
        let chunk_data =  autonomi_chunk::Chunk::new(bytes);
        self.create_chunk_raw(chunk_data, evm_wallet, cache_only).await
    }

    pub async fn create_chunk(&self, chunk: Chunk, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Chunk, ChunkError> {
        let content = match chunk.content.clone() {
            Some(content) => content,
            None => return Err(ChunkError::CreateError(CreateError::InvalidData("Empty chunk payload".to_string())))
        };
        let decoded_content = BASE64_STANDARD.decode(content).unwrap_or_else(|_| Vec::new());
        let chunk_data =  autonomi_chunk::Chunk::new(Bytes::from(decoded_content.clone()));

        self.create_chunk_raw(chunk_data, evm_wallet, cache_only).await
    }

    pub async fn create_chunk_raw(&self, chunk: autonomi_chunk::Chunk, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Chunk, ChunkError> {
        info!("Create chunk at address [{}]", chunk.address.to_hex());
        let chunk_address = self.caching_client.chunk_put(&chunk, PaymentOption::from(&evm_wallet), cache_only).await?;
        info!("Queued command to create chunk at [{}]", chunk_address.to_hex());
        Ok(Chunk::new(None, Some(chunk_address.to_hex())))
    }

    pub async fn get_chunk_binary(&self, address: String) -> Result<autonomi::Chunk, ChunkError> {
        match ChunkAddress::from_hex(address.as_str()) {
            Ok(chunk_address) => match self.caching_client.chunk_get_internal(&chunk_address).await {
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
            Ok(chunk_address) => match self.caching_client.chunk_get_internal(&chunk_address).await {
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