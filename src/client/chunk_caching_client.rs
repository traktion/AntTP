use ant_core::data::{DataChunk, XorName};
use async_trait::async_trait;
use bytes::Bytes;
use chunk_streamer::chunk_streamer::ChunkGetter;
use log::{debug, error, info};
use mockall::mock;
use mockall_double::double;
#[double]
use crate::client::CachingClient;
use crate::client::command::chunk::create_chunk_command::CreateChunkCommand;
use crate::error::chunk_error::ChunkError;
use crate::controller::StoreType;

#[derive(Clone)]
pub struct ChunkCachingClient {
    caching_client: CachingClient,
}

mock! {
    pub ChunkCachingClient {
        pub fn new(caching_client: CachingClient) -> Self;
        pub async fn chunk_put(
            &self,
            chunk: &DataChunk,
            store_type: StoreType
        ) -> Result<XorName, ChunkError>;
        pub async fn chunk_get_internal(&self, address: &XorName) -> Result<DataChunk, ChunkError>;
    }
    impl Clone for ChunkCachingClient {
        fn clone(&self) -> Self;
    }
    #[async_trait]
    impl ChunkGetter for ChunkCachingClient {
        async fn chunk_get(&self, address: &XorName) -> ant_core::data::error::Result<Option<DataChunk>>;
    }
}

impl ChunkCachingClient {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn chunk_put(
        &self,
        chunk: &DataChunk,
        store_type: StoreType
    ) -> Result<XorName, ChunkError> {
        self.caching_client.get_hybrid_cache().insert(hex::encode(chunk.address), Vec::from(chunk.content.clone()));
        debug!("creating chunk with address [{}] in cache", hex::encode(chunk.address));
        if store_type == StoreType::Network {
            let command = Box::new(
                CreateChunkCommand::new(self.caching_client.get_client_harness().clone(), chunk.clone())
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(chunk.address)
    }

    pub async fn chunk_get_internal(&self, address: &XorName) -> Result<DataChunk, ChunkError> {
        let local_address = address.clone();
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().get_or_fetch(&hex::encode(local_address), {
            let client = self.caching_client.get_client_harness().get_ref().lock().await.get_client().await?;
            || async move {
                match client.chunk_get(&local_address).await {
                    Ok(Some(chunk)) => {
                        info!("retrieved chunk for [{}] from network - storing in hybrid cache", hex::encode(local_address));
                        Ok(Vec::from(chunk.content))
                    }
                    Ok(None) => {
                        error!("Failed to retrieve chunk for [{}] from network", hex::encode(local_address));
                        Err(anyhow::anyhow!(format!("Failed to retrieve chunk for [{}] from network", hex::encode(local_address))))
                    }
                    Err(err) => {
                        error!("Failed to retrieve chunk for [{}] from network {:?}", hex::encode(local_address), err);
                        Err(anyhow::anyhow!(format!("Failed to retrieve chunk for [{}] from network {:?}", hex::encode(local_address), err)))
                    }
                }
            }
        }).await?;
        info!("retrieved chunk for [{}] from hybrid cache", hex::encode(address));
        Ok(DataChunk::new(address.clone(), Bytes::from(cache_entry.value().to_vec())))
    }
}

#[async_trait]
impl ChunkGetter for ChunkCachingClient {
    async fn chunk_get(&self, address: &XorName) -> ant_core::data::error::Result<Option<DataChunk>> {
        match self.chunk_get_internal(address).await {
            Ok(chunk) => Ok(Some(chunk)),
            Err(_) => Err(ant_core::data::error::Error::InvalidData("error getting chunk".to_string())),
        }
    }
}