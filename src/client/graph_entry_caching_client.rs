use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::{GraphEntry, GraphEntryAddress};
use log::{debug, info};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;
use crate::client::command::graph::create_graph_entry_command::CreateGraphEntryCommand;
use crate::client::command::graph::get_graph_entry_command::GetGraphEntryCommand;
use crate::client::error::{GetError, GraphError};
use crate::controller::CacheType;

impl CachingClient {

    pub async fn graph_entry_put(
        &self,
        graph_entry: GraphEntry,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(AttoTokens, GraphEntryAddress), GraphError> {
        self.cache_graph_entry(graph_entry.clone(), cache_only.clone());
        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(CreateGraphEntryCommand::new(self.client_harness.clone(), graph_entry.clone(), payment_option))
            ).await.unwrap();
        }
        Ok((AttoTokens::zero(), graph_entry.address()))
    }

    fn cache_graph_entry(&self, graph_entry: GraphEntry, cache_only: Option<CacheType>) {
        let ttl = if cache_only.is_some() { u64::MAX } else { self.ant_tp_config.cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(graph_entry.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize graph entry");
        info!("updating cache with graph_entry at address gg[{}] and TTL [{}]", graph_entry.address().to_hex(), ttl);
        if cache_only.is_some_and(|v| matches!(v, CacheType::Disk)) {
            self.hybrid_cache.insert(format!("gg{}", graph_entry.address().to_hex()), serialised_cache_item);
        } else {
            self.hybrid_cache.memory().insert(format!("gg{}", graph_entry.address().to_hex()), serialised_cache_item);
        }
    }

    pub async fn graph_entry_get(
        &self,
        address: &GraphEntryAddress,
    ) -> Result<GraphEntry, GraphError> {
        let local_address = address.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("gg{}", local_address.to_hex()), {
            let client = match self.client_harness.get_ref().lock().await.get_client().await {
                Some(client) => client,
                None => return Err(GraphError::GetError(GetError::NetworkOffline(
                    format!("Failed to retrieve chunk for [{}] as offline network", local_address.to_hex()))))
            };
            
            || async move {
                match client.graph_entry_get(&local_address).await {
                    Ok(scratchpad) => {
                        debug!("found graph entry for address [{}]", local_address.to_hex());
                        let cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                        Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize graph entry"))
                    }
                    Err(_) => Err(foyer::Error::other(format!("Failed to retrieve graph entry for [{}] from network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<GraphEntry> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize graph entry");
                info!("retrieved graph entry for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    self.command_executor.send(
                        Box::new(GetGraphEntryCommand::new(self.client_harness.clone(), self.hybrid_cache.clone(), address.clone(), self.ant_tp_config.cached_mutable_ttl))
                    ).await.unwrap();
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(e) => Err(GraphError::GetError(GetError::RecordNotFound(e.to_string()))),
        }
    }
}