use autonomi::client::payment::PaymentOption;
use autonomi::{GraphEntry, GraphEntryAddress};
use log::{debug, info};
use mockall_double::double;
use crate::client::cache_item::CacheItem;
#[double]
use crate::client::CachingClient;
use crate::client::GRAPH_ENTRY_CACHE_KEY;
use crate::client::command::graph::create_graph_entry_command::CreateGraphEntryCommand;
use crate::client::command::graph::get_graph_entry_command::GetGraphEntryCommand;
use crate::error::graph_error::GraphError;
use crate::controller::StoreType;


use mockall::mock;

#[derive(Debug, Clone)]
pub struct GraphEntryCachingClient {
    caching_client: CachingClient,
}

mock! {
    #[derive(Debug)]
    pub GraphEntryCachingClient {
        pub fn new(caching_client: CachingClient) -> Self;
        pub async fn graph_entry_put(
            &self,
            graph_entry: GraphEntry,
            payment_option: PaymentOption,
            store_type: StoreType,
        ) -> Result<GraphEntryAddress, GraphError>;
        pub async fn graph_entry_get(
            &self,
            address: &GraphEntryAddress,
        ) -> Result<GraphEntry, GraphError>;
    }
    impl Clone for GraphEntryCachingClient {
        fn clone(&self) -> Self;
    }
}

impl GraphEntryCachingClient {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn graph_entry_put(
        &self,
        graph_entry: GraphEntry,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<GraphEntryAddress, GraphError> {
        self.cache_graph_entry(graph_entry.clone(), store_type.clone());
        if store_type == StoreType::Network {
            let command = Box::new(
                CreateGraphEntryCommand::new(self.caching_client.get_client_harness().clone(), graph_entry.clone(), payment_option)
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(graph_entry.address())
    }

    fn cache_graph_entry(&self, graph_entry: GraphEntry, store_type: StoreType) {
        let ttl = if store_type != StoreType::Network { u64::MAX } else { self.caching_client.get_ant_tp_config().cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(graph_entry.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize graph entry");
        info!("updating cache with graph_entry at address {}[{}] and TTL [{}]", GRAPH_ENTRY_CACHE_KEY, graph_entry.address().to_hex(), ttl);
        if store_type == StoreType::Disk {
            self.caching_client.get_hybrid_cache().insert(format!("{}{}", GRAPH_ENTRY_CACHE_KEY, graph_entry.address().to_hex()), serialised_cache_item);
        } else {
            self.caching_client.get_hybrid_cache().memory().insert(format!("{}{}", GRAPH_ENTRY_CACHE_KEY, graph_entry.address().to_hex()), serialised_cache_item);
        }
    }

    pub async fn graph_entry_get(
        &self,
        address: &GraphEntryAddress,
    ) -> Result<GraphEntry, GraphError> {
        let local_address = address.clone();
        let local_ant_tp_config = self.caching_client.get_ant_tp_config().clone();
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().fetch(format!("{}{}", GRAPH_ENTRY_CACHE_KEY, local_address.to_hex()), {
            let client = self.caching_client.get_client_harness().get_ref().lock().await.get_client().await?;
            || async move {
                match client.graph_entry_get(&local_address).await {
                    Ok(scratchpad) => {
                        debug!("found graph entry for address [{}]", local_address.to_hex());
                        let cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                        match rmp_serde::to_vec(&cache_item) {
                            Ok(cache_item) => Ok(cache_item),
                            Err(e) => Err(foyer::Error::other(format!("Failed to serialize graph entry for [{}]: {}", local_address.to_hex(), e.to_string())))
                        }
                    }
                    Err(e) => Err(foyer::Error::other(format!("Failed to retrieve graph entry for [{}] from network: {}", local_address.to_hex(), e.to_string())))
                }
            }
        }).await?;
        let cache_item: CacheItem<GraphEntry> = rmp_serde::from_slice(cache_entry.value())?;
        info!("retrieved graph entry for [{}] from hybrid cache", address.to_hex());
        if cache_item.has_expired() {
            let command = Box::new(
                GetGraphEntryCommand::new(self.caching_client.get_client_harness().clone(), self.caching_client.get_hybrid_cache().clone(), address.clone(), self.caching_client.get_ant_tp_config().cached_mutable_ttl)
            );
            self.caching_client.send_get_command(command).await?;
        }
        Ok(cache_item.item.unwrap())
    }
}