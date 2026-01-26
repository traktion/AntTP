use async_trait::async_trait;
use autonomi::{Chunk, ChunkAddress};
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use chunk_streamer::chunk_streamer::ChunkGetter;
use log::{debug, error, info};
use mockall::mock;
use crate::client::CachingClient;
use crate::client::command::chunk::create_chunk_command::CreateChunkCommand;
use crate::error::chunk_error::ChunkError;
use crate::controller::StoreType;

#[derive(Debug, Clone)]
pub struct ChunkCachingClient {
    caching_client: CachingClient,
}

mock! {
    #[derive(Debug)]
    pub ChunkCachingClient {
        pub fn new(caching_client: CachingClient) -> Self;
        pub async fn chunk_put(
            &self,
            chunk: &Chunk,
            payment_option: PaymentOption,
            store_type: StoreType
        ) -> Result<ChunkAddress, ChunkError>;
        pub async fn chunk_get_internal(&self, address: &ChunkAddress) -> Result<Chunk, ChunkError>;
    }
    impl Clone for ChunkCachingClient {
        fn clone(&self) -> Self;
    }
}

impl ChunkCachingClient {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn chunk_put(
        &self,
        chunk: &Chunk,
        payment_option: PaymentOption,
        store_type: StoreType
    ) -> Result<ChunkAddress, ChunkError> {
        self.caching_client.hybrid_cache.insert(chunk.address.to_hex(), Vec::from(chunk.value.clone()));
        debug!("creating chunk with address [{}] in cache", chunk.address.to_hex());
        if store_type == StoreType::Network {
            let command = Box::new(
                CreateChunkCommand::new(self.caching_client.client_harness.clone(), chunk.clone(), payment_option)
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(chunk.address)
    }

    pub async fn chunk_get_internal(&self, address: &ChunkAddress) -> Result<Chunk, ChunkError> {
        let local_address = address.clone();
        let cache_entry = self.caching_client.hybrid_cache.get_ref().fetch(local_address.to_hex(), {
            let client = self.caching_client.client_harness.get_ref().lock().await.get_client().await?;
            || async move {
                match client.chunk_get(&local_address).await {
                    Ok(chunk) => {
                        info!("retrieved chunk for [{}] from network - storing in hybrid cache", local_address.to_hex());
                        Ok(Vec::from(chunk.value))
                    }
                    Err(err) => {
                        error!("Failed to retrieve chunk for [{}] from network {:?}", local_address.to_hex(), err);
                        Err(foyer::Error::other(format!("Failed to retrieve chunk for [{}] from network {:?}", local_address.to_hex(), err)))
                    }
                }
            }
        }).await?;
        info!("retrieved chunk for [{}] from hybrid cache", address.to_hex());
        Ok(Chunk::new(Bytes::from(cache_entry.value().to_vec())))
    }
}

#[async_trait]
impl ChunkGetter for ChunkCachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, autonomi::client::GetError> {
        match self.chunk_get_internal(address).await {
            Ok(chunk) => Ok(chunk),
            Err(_) => Err(autonomi::client::GetError::RecordNotFound)
        }
    }
}

#[async_trait]
impl ChunkGetter for CachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, autonomi::client::GetError> {
        let chunk_caching_client = ChunkCachingClient::new(self.clone());
        chunk_caching_client.chunk_get(address).await
    }
}