use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::client::{GetError, PutError};
use autonomi::data::DataAddress;
use autonomi::files::PublicArchive;
use bytes::Bytes;
use chunk_streamer::chunk_encrypter::ChunkEncrypter;
use log::{debug, info, warn};
use crate::client::CachingClient;

impl CachingClient {

    pub async fn data_put_public(
        &self,
        data: Bytes,
        payment_option: PaymentOption,
        is_cache_only: bool,
    ) -> Result<(AttoTokens, DataAddress), PutError> {
        // todo: avoid double encrypting on upload?
        let chunk_encrypter = ChunkEncrypter::new();
        match chunk_encrypter.encrypt(true, data.clone()).await {
            Ok((chunks, data_map_chunk)) => {
                let data_map_addr = *data_map_chunk.0.address();
                info!("Uploading datamap chunk to the cache: {data_map_addr:?}");
                let data_address = DataAddress::new(*data_map_addr.xorname());

                let mut i = 0;
                for chunk in chunks {
                    //let prefix = if i == 0 { "pd" } else { "" };
                    debug!("Caching chunk at address: {}", chunk.address.to_hex());
                    self.hybrid_cache.insert(format!("{}", chunk.address.to_hex()), chunk.value.to_vec());
                    //i = i + 1;
                }

                if is_cache_only {
                    Ok((AttoTokens::zero(), data_address))
                } else {
                    match self.client_harness.get_ref().lock().await.get_client().await {
                        Some(client) => {
                            client.data_put_public(data, payment_option).await
                        },
                        None => Err(PutError::Serialization(format!("network offline")))
                    }
                }
            },
            Err(err) => Err(err)
        }
    }

    pub async fn data_get_public(&self, addr: &DataAddress) -> Result<Bytes, GetError> {
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(format!("pd{}", local_address.to_hex()), || async move {
            // todo: optimise range_to to first chunk length (to avoid downloading other chunks when not needed)
            let maybe_bytes = local_caching_client.download_stream(local_address, 0, 524288).await;
            match maybe_bytes {
                Ok(bytes) => {
                    match PublicArchive::from_bytes(bytes.clone()) {
                        // confirm that serialisation can be successful, before returning the data
                        Ok(_) => {
                            info!("retrieved public archive for [{}] from network - storing in hybrid cache", local_address.to_hex());
                            info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                            Ok(Vec::from(bytes))
                        },
                        Err(err) => {
                            warn!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), err);
                            Err(foyer::Error::other(format!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), err)))
                        }
                    }
                },
                Err(err) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network {:?}", local_address.to_hex(), err)))
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved public archive for [{}] from hybrid cache", addr.to_hex());
                Ok(Bytes::from(cache_entry.value().to_vec()))
            },
            Err(_) => Err(GetError::RecordNotFound),
        }
    }
}