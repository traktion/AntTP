use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::{GraphEntry, GraphEntryAddress};
use autonomi::graph::GraphError;
use log::{debug, info, warn};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;

impl CachingClient {

    pub async fn graph_entry_put(
        &self,
        entry: GraphEntry,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, GraphEntryAddress), GraphError> {
        let address = entry.address();
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("creating graph entry async");
                    client.graph_entry_put(entry, payment_option).await
                });
                Ok((AttoTokens::zero(), address))
            },
            None => Err(GraphError::Serialization(format!("network offline")))
        }
    }

    pub async fn graph_entry_get(
        &self,
        address: &GraphEntryAddress,
    ) -> Result<GraphEntry, GraphError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("gg{}", local_address.to_hex()), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(client) => {
                        match client.graph_entry_get(&local_address).await {
                            Ok(scratchpad) => {
                                debug!("found graph entry for address [{}]", local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                let cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                                Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize graph entry"))
                            }
                            Err(_) => Err(foyer::Error::other(format!("Failed to retrieve graph entry for [{}] from network", local_address.to_hex())))
                        }
                    },
                    None => Err(foyer::Error::other(format!("Failed to retrieve graph entry for [{}] from offline network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<GraphEntry> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize graph entry");
                info!("retrieved graph entry for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    // update cache in the background
                    let local_address = address.clone();
                    let local_hybrid_cache = self.hybrid_cache.clone();
                    tokio::spawn({
                        let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
                        async move {
                            match maybe_local_client {
                                Some(client) => {
                                    info!("refreshing hybrid cache with graph entry for [{}] from network, timestamp [{}], ttl [{}]", local_address.to_hex(), cache_item.timestamp, cache_item.ttl);
                                    match client.graph_entry_get(&local_address).await {
                                        Ok(scratchpad) => {
                                            let new_cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                                            local_hybrid_cache.insert(
                                                format!("gg{}", local_address.to_hex()),
                                                rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize graph entry")
                                            );
                                            info!("inserted hybrid cache with graph entry for [{}] from network", local_address.to_hex());
                                        }
                                        Err(e) => warn!("Failed to refresh expired graph entry for [{}] from network [{}]", local_address.to_hex(), e)
                                    }
                                },
                                None => warn!("Failed to refresh expired graph entry for [{}] from offline network", local_address.to_hex())
                            }
                        }
                    });
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(GraphError::Serialization(format!("network offline"))),
        }
    }
}