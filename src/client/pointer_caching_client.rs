use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::pointer::{PointerError, PointerTarget};
use autonomi::{Pointer, PointerAddress, SecretKey};
use autonomi::client::GetError;
use log::{debug, info, warn};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;
use crate::controller::CacheType;
use crate::command::pointer::create_pointer_command::CreatePointerCommand;
use crate::command::pointer::update_pointer_command::UpdatePointerCommand;

impl CachingClient {

    pub async fn pointer_create(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(AttoTokens, PointerAddress), PointerError> {
        let pointer = self.cache_pointer(owner, &target, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(CreatePointerCommand::new(self.client_harness.clone(), owner.clone(), target, payment_option))
            ).await.unwrap();
        }
        Ok((AttoTokens::zero(), pointer.address()))
    }

    pub async fn pointer_update(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
        cache_only: Option<CacheType>,
    ) -> Result<(), PointerError> {
        self.cache_pointer(owner, &target, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(UpdatePointerCommand::new(self.client_harness.clone(), owner.clone(), target))
            ).await.unwrap();
        }
        Ok(())
    }

    fn cache_pointer(&self, owner: &SecretKey, target: &PointerTarget, cache_only: Option<CacheType>) -> Pointer {
        let pointer = Pointer::new(owner, 0, target.clone());
        let ttl = if cache_only.is_some() { u64::MAX } else { self.ant_tp_config.cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(pointer.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer");
        info!("updating cache with pointer at address pg[{}] to target [{}] and TTL [{}]", pointer.address().to_hex(), target.to_hex(), ttl);
        if cache_only.is_some_and(|v| matches!(v, CacheType::Disk)) {
            self.hybrid_cache.insert(format!("pg{}", pointer.address().to_hex()), serialised_cache_item);
        } else {
            self.hybrid_cache.memory().insert(format!("pg{}", pointer.address().to_hex()), serialised_cache_item);
        }
        pointer
    }

    pub async fn pointer_get(&self, address: &PointerAddress) -> Result<Pointer, PointerError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("pg{}", local_address.to_hex()), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(client) => {
                        match client.pointer_get(&local_address).await {
                            Ok(pointer) => {
                                debug!("found pointer [{}] for address [{}]", hex::encode(pointer.target().to_hex()), local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                let cache_item = CacheItem::new(Some(pointer.clone()), local_ant_tp_config.cached_mutable_ttl);
                                Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer"))
                            },
                            Err(_) => Err(foyer::Error::other(format!("Failed to retrieve pointer for [{}] from network", local_address.to_hex())))
                        }
                    },
                    None => Err(foyer::Error::other(format!("Failed to retrieve pointer for [{}] from offline network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<Pointer> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize pointer");
                info!("retrieved pointer for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    // update cache in the background
                    let local_address = address.clone();
                    let local_hybrid_cache = self.hybrid_cache.clone();
                    tokio::spawn({
                        let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
                        async move {
                            match maybe_local_client {
                                Some(client) => {
                                    info!("refreshing hybrid cache with pointer for [{}] from network, timestamp [{}], ttl [{}]", local_address.to_hex(), cache_item.timestamp, cache_item.ttl);
                                    match client.pointer_get(&local_address).await {
                                        Ok(pointer) => {
                                            let new_cache_item = CacheItem::new(Some(pointer.clone()), local_ant_tp_config.cached_mutable_ttl);
                                            local_hybrid_cache.insert(
                                                format!("pg{}", local_address.to_hex()),
                                                rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize pointer")
                                            );
                                            info!("inserted hybrid cache with pointer for [{}] from network", local_address.to_hex());
                                        },
                                        Err(e) => warn!("Failed to refresh expired pointer for [{}] from network [{}]", local_address.to_hex(), e)
                                    }
                                },
                                None => warn!("Failed to refresh expired pointer for [{}] from offline network", local_address.to_hex())
                            }
                        }
                    });
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(PointerError::GetError(GetError::RecordNotFound)),
        }
    }

    pub async fn pointer_check_existence(&self, address: &PointerAddress) -> Result<bool, PointerError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("pce{}", local_address.to_hex()), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(client) => {
                        match client.pointer_check_existence(&local_address).await {
                            Ok(_) => {
                                debug!("pointer exists for address [{}]", local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                let cache_item = CacheItem::new(Some(true), local_ant_tp_config.cached_mutable_ttl);
                                Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer"))
                            },
                            Err(_) => Err(foyer::Error::other(format!("Failed to pointer check existence for [{}] from network", local_address.to_hex())))
                        }
                    },
                    None => Err(foyer::Error::other(format!("Failed to pointer check existence for [{}] from offline network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<bool> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize pointer");
                info!("retrieved pointer check existence for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    // update cache in the background
                    let local_address = address.clone();
                    let local_hybrid_cache = self.hybrid_cache.clone();
                    tokio::spawn({
                        let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
                        async move {
                            match maybe_local_client {
                                Some(client) => {
                                    info!("refreshing hybrid cache with pointer check existence for [{}] from network, timestamp [{}], ttl [{}]", local_address.to_hex(), cache_item.timestamp, cache_item.ttl);
                                    match client.pointer_get(&local_address).await {
                                        Ok(pointer) => {
                                            let new_cache_item = CacheItem::new(Some(pointer.clone()), local_ant_tp_config.cached_mutable_ttl);
                                            local_hybrid_cache.insert(
                                                format!("pce{}", local_address.to_hex()),
                                                rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize pointer")
                                            );
                                            info!("inserted hybrid cache with pointer check existence for [{}] from network", local_address.to_hex());
                                        },
                                        Err(e) => warn!("Failed to refresh expired pointer check existence for [{}] from network [{}]", local_address.to_hex(), e)
                                    }
                                },
                                None => warn!("Failed to refresh expired pointer check existence for [{}] from offline network", local_address.to_hex())
                            }
                        }
                    });
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(PointerError::GetError(GetError::RecordNotFound)),
        }
    }
}