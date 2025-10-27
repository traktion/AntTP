use ant_evm::AttoTokens;
use async_trait::async_trait;
use autonomi::{Chunk, ChunkAddress};
use autonomi::client::payment::PaymentOption;
use bytes::Bytes;
use chunk_streamer::chunk_streamer::ChunkGetter;
use log::{debug, error, info};
use crate::client::CachingClient;
use crate::client::command::chunk::create_chunk_command::CreateChunkCommand;
use crate::client::error::{ChunkError, GetError};
use crate::controller::CacheType;

#[async_trait]
impl ChunkGetter for CachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, autonomi::client::GetError> {
        match self.chunk_get_internal(address).await {
            Ok(chunk) => Ok(chunk),
            Err(_) => Err(autonomi::client::GetError::RecordNotFound)
        }
    }
}

impl CachingClient {
    pub async fn chunk_put(
        &self,
        chunk: &Chunk,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>
    ) -> Result<(AttoTokens, ChunkAddress), ChunkError> {
        self.hybrid_cache.insert(chunk.address.to_hex(), Vec::from(chunk.value.clone()));
        debug!("creating chunk with address [{}] in cache", chunk.address.to_hex());
        if !cache_only.is_some() {
            let command = Box::new(
                CreateChunkCommand::new(self.client_harness.clone(), chunk.clone(), payment_option)
            );
            self.send_create_command(command).await?;
        }
        Ok((AttoTokens::zero(), chunk.address))
    }

    pub async fn chunk_get_internal(&self, address: &ChunkAddress) -> Result<Chunk, ChunkError> {
        let local_address = address.clone();
        match self.hybrid_cache.get_ref().fetch(local_address.to_hex(), {
            let client = match self.client_harness.get_ref().lock().await.get_client().await {
                Some(client) => client,
                None => return Err(GetError::NetworkOffline(
                    format!("Failed to retrieve chunk for [{}] as offline network", local_address.to_hex())).into())
            };

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
            }}).await {
            Ok(cache_entry) => {
                info!("retrieved chunk for [{}] from hybrid cache", address.to_hex());
                Ok(Chunk::new(Bytes::from(cache_entry.value().to_vec())))
            },
            Err(e) => Err(ChunkError::GetError(e.into()))
        }
    }
}