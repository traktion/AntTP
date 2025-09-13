use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::register::{RegisterAddress, RegisterError, RegisterHistory, RegisterValue};
use autonomi::SecretKey;
use log::{debug, info, warn};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;

impl CachingClient {

    pub async fn register_create(
        &self,
        owner: &SecretKey,
        initial_value: RegisterValue,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, RegisterAddress), RegisterError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let owner_clone = owner.clone();
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("creating register async");
                    client.register_create(&owner_clone, initial_value, payment_option).await
                });
                Ok((AttoTokens::zero(), RegisterAddress::new(owner.clone().public_key())))
            },
            None => Err(RegisterError::InvalidCost) // todo: improve error type
        }
    }

    pub async fn register_update(
        &self,
        owner: &SecretKey,
        new_value: RegisterValue,
        payment_option: PaymentOption,
    ) -> Result<AttoTokens, RegisterError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let owner_clone = owner.clone();
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("updating register async");
                    client.register_update(&owner_clone, new_value, payment_option).await
                });
                Ok(AttoTokens::zero())
            },
            None => Err(RegisterError::InvalidCost) // todo: improve error type
        }
    }

    pub async fn register_get(&self, address: &RegisterAddress) -> Result<RegisterValue, RegisterError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("rg{}", local_address.to_hex()), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(client) => {
                        match client.register_get(&local_address).await {
                            Ok(register_value) => {
                                debug!("found register value [{}] for address [{}] from network", hex::encode(register_value.clone()), local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                let cache_item = CacheItem::new(Some(register_value.clone()), local_ant_tp_config.cached_mutable_ttl);
                                Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize register"))
                            }
                            Err(_) => Err(foyer::Error::other(format!("Failed to retrieve register for [{}] from network", local_address.to_hex())))
                        }
                    },
                    None => Err(foyer::Error::other(format!("Failed to retrieve register for [{}] from offline network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<RegisterValue> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize register");
                info!("retrieved register for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    // update cache in the background
                    let local_address = address.clone();
                    let local_hybrid_cache = self.hybrid_cache.clone();
                    tokio::spawn({
                        let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
                        async move {
                            match maybe_local_client {
                                Some(client) => {
                                    info!("refreshing hybrid cache with register for [{}] from network, timestamp [{}], ttl [{}]", local_address.to_hex(), cache_item.timestamp, cache_item.ttl);
                                    match client.register_get(&local_address).await {
                                        Ok(register_value) => {
                                            let new_cache_item = CacheItem::new(Some(register_value.clone()), local_ant_tp_config.cached_mutable_ttl);
                                            local_hybrid_cache.insert(
                                                format!("rg{}", local_address.to_hex()),
                                                rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize register")
                                            );
                                            info!("inserted hybrid cache with register for [{}] from network", local_address.to_hex());
                                        }
                                        Err(e) => warn!("Failed to refresh expired register for [{}] from network [{}]", local_address.to_hex(), e)
                                    }
                                },
                                None => warn!("Failed to refresh expired register for [{}] from offline network", local_address.to_hex())
                            }
                        }
                    });
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(RegisterError::CannotUpdateNewRegister),
        }
    }

    pub async fn register_history(&self, addr: &RegisterAddress) -> RegisterHistory {
        self.client_harness.get_ref().lock().await.get_client().await.expect("network offline").register_history(addr)
    }
}