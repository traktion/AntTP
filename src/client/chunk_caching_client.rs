use ant_evm::AttoTokens;
use async_trait::async_trait;
use autonomi::{Chunk, ChunkAddress};
use autonomi::client::payment::PaymentOption;
use autonomi::client::{GetError, PutError};
use bytes::Bytes;
use chunk_streamer::chunk_streamer::ChunkGetter;
use log::{debug, error, info};
use crate::client::CachingClient;
use crate::controller::CacheType;

#[async_trait]
impl ChunkGetter for CachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, GetError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(local_address.to_hex(), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(local_client) => {
                        match local_client.chunk_get(&local_address).await {
                            Ok(chunk) => {
                                info!("retrieved chunk for [{}] from network - storing in hybrid cache", local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                Ok(Vec::from(chunk.value))
                            }
                            Err(err) => {
                                error!("Failed to retrieve chunk for [{}] from network {:?}", local_address.to_hex(), err);
                                Err(foyer::Error::other(format!("Failed to retrieve chunk for [{}] from network {:?}", local_address.to_hex(), err)))
                            }
                        }
                    },
                    None => {
                        error!("Failed to retrieve chunk for [{}] as offline network", local_address.to_hex());
                        Err(foyer::Error::other(format!("Failed to retrieve chunk for [{}] from offline network", local_address.to_hex())))
                    }
                }
            }}).await {
            Ok(cache_entry) => {
                info!("retrieved chunk for [{}] from hybrid cache", address.to_hex());
                Ok(Chunk::new(Bytes::from(cache_entry.value().to_vec())))
            },
            Err(_) => Err(GetError::RecordNotFound)
        }
    }
}

impl CachingClient {
    pub async fn chunk_put(
        &self,
        chunk: &Chunk,
        payment_option: PaymentOption,
        is_cache_only: Option<CacheType>
    ) -> Result<(AttoTokens, ChunkAddress), PutError> {
        self.hybrid_cache.insert(chunk.address.to_hex(), Vec::from(chunk.value.clone()));
        debug!("creating chunk with address [{}] in cache", chunk.address.to_hex());
        if is_cache_only.is_some() {
            Ok((AttoTokens::zero(), chunk.address))
        } else {
            match self.client_harness.get_ref().lock().await.get_client().await {
                Some(client) => {
                    // todo: move to job processor
                    let local_chunk = chunk.clone();
                    tokio::spawn(async move {
                        debug!("creating chunk with address [{}] on network", local_chunk.address.to_hex());
                        client.chunk_put(&local_chunk, payment_option).await
                    });
                    Ok((AttoTokens::zero(), chunk.address))
                },
                None => Err(PutError::Serialization(format!("network offline")))
            }
        }
    }
}