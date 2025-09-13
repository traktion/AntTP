use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::{Scratchpad, ScratchpadAddress, SecretKey};
use autonomi::scratchpad::ScratchpadError;
use bytes::Bytes;
use log::{debug, info, warn};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;

impl CachingClient {
    
    pub async fn scratchpad_create(
        &self,
        owner: &SecretKey,
        content_type: u64,
        initial_data: &Bytes,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, ScratchpadAddress), ScratchpadError> {
        let owner_clone = owner.clone();
        let initial_data_clone = initial_data.clone();
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("creating scratchpad async");
                    client.scratchpad_create(&owner_clone, content_type, &initial_data_clone, payment_option).await
                });
                let address = ScratchpadAddress::new(owner.public_key());
                Ok((AttoTokens::zero(), address))
            },
            None => Err(ScratchpadError::Serialization)
        }
    }

    pub async fn scratchpad_create_public(
        &self,
        owner: &SecretKey,
        content_type: u64,
        initial_data: &Bytes,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, ScratchpadAddress), ScratchpadError> {
        let address = ScratchpadAddress::new(owner.public_key());
        let already_exists = self.scratchpad_check_existance(&address).await?;
        if already_exists {
            return Err(ScratchpadError::ScratchpadAlreadyExists(address));
        }

        let counter = 0;
        let signature = owner.sign(Scratchpad::bytes_for_signature(
            address,
            content_type,
            &initial_data.clone(),
            counter,
        ));
        let scratchpad = Scratchpad::new_with_signature(owner.public_key(), content_type, initial_data.clone(), counter, signature);
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                tokio::spawn(async move {
                    debug!("creating scratchpad async");
                    client.scratchpad_put(scratchpad, payment_option).await
                });
                Ok((AttoTokens::zero(), address))
            },
            None => Err(ScratchpadError::Serialization)
        }
    }

    pub async fn scratchpad_update_public(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        counter: u64,
    ) -> Result<(), ScratchpadError> {
        let address = ScratchpadAddress::new(owner.public_key());

        let version = counter + 1;
        let signature = owner.sign(Scratchpad::bytes_for_signature(
            address,
            content_type,
            &data.clone(),
            version,
        ));
        let scratchpad = Scratchpad::new_with_signature(owner.public_key(), content_type, data.clone(), version, signature);
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                tokio::spawn(async move {
                    debug!("creating scratchpad async");
                    client.scratchpad_put(scratchpad, payment_option).await
                });
                Ok(())
            },
            None => Err(ScratchpadError::Serialization)
        }
    }

    pub async fn scratchpad_check_existance(
        &self,
        address: &ScratchpadAddress,
    ) -> Result<bool, ScratchpadError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => client.scratchpad_check_existence(address).await,
            None => Err(ScratchpadError::Serialization),
        }
    }

    pub async fn scratchpad_update(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
    ) -> Result<(), ScratchpadError> {
        let owner_clone = owner.clone();
        let data_clone = data.clone();
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("updating scratchpad async");
                    client.scratchpad_update(&owner_clone, content_type, &data_clone).await
                });
                Ok(())
            },
            None => Err(ScratchpadError::Serialization)
        }
    }

    pub async fn scratchpad_get(&self, address: &ScratchpadAddress) -> Result<Scratchpad, ScratchpadError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("sg{}", local_address.to_hex()), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(client) => {
                        match client.scratchpad_get(&local_address).await {
                            Ok(scratchpad) => {
                                debug!("found scratchpad for address [{}]", local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                let cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                                Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize scratchpad"))
                            }
                            Err(_) => Err(foyer::Error::other(format!("Failed to retrieve scratchpad for [{}] from network", local_address.to_hex())))
                        }
                    },
                    None => Err(foyer::Error::other(format!("Failed to retrieve scratchpad for [{}] from offline network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<Scratchpad> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize scratchpad");
                info!("retrieved scratchpad for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    // update cache in the background
                    let local_address = address.clone();
                    let local_hybrid_cache = self.hybrid_cache.clone();
                    tokio::spawn({
                        let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
                        async move {
                            match maybe_local_client {
                                Some(client) => {
                                    info!("refreshing hybrid cache with scratchpad for [{}] from network, timestamp [{}], ttl [{}]", local_address.to_hex(), cache_item.timestamp, cache_item.ttl);
                                    match client.scratchpad_get(&local_address).await {
                                        Ok(scratchpad) => {
                                            let new_cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                                            local_hybrid_cache.insert(
                                                format!("sg{}", local_address.to_hex()),
                                                rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize scratchpad")
                                            );
                                            info!("inserted hybrid cache with scratchpad for [{}] from network", local_address.to_hex());
                                        }
                                        Err(e) => warn!("Failed to refresh expired scratchpad for [{}] from network [{}]", local_address.to_hex(), e)
                                    }
                                },
                                None => warn!("Failed to refresh expired scratchpad for [{}] from offline network", local_address.to_hex())
                            }
                        }
                    });
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(ScratchpadError::Serialization),
        }
    }
}