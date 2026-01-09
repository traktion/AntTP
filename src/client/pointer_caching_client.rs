use autonomi::client::payment::PaymentOption;
use autonomi::pointer::PointerTarget;
use autonomi::{Pointer, PointerAddress, SecretKey};
use log::{debug, info, warn};
use crate::client::cache_item::CacheItem;
use crate::client::{CachingClient, POINTER_CACHE_KEY, POINTER_CHECK_CACHE_KEY};
use crate::client::command::pointer::check_pointer_command::CheckPointerCommand;
use crate::client::command::pointer::get_pointer_command::GetPointerCommand;
use crate::controller::CacheType;
use crate::client::command::pointer::create_pointer_command::CreatePointerCommand;
use crate::client::command::pointer::update_pointer_command::UpdatePointerCommand;
use crate::error::GetError;
use crate::error::pointer_error::PointerError;

impl CachingClient {

    pub async fn pointer_create(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
        counter: Option<u64>,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<PointerAddress, PointerError> {
        let pointer = self.cache_pointer(owner, &target, counter, cache_only.clone());

        if !cache_only.is_some() {
            let command = Box::new(
                CreatePointerCommand::new(self.client_harness.clone(), owner.clone(), target, payment_option)
            );
            self.send_create_command(command).await?;
        }
        Ok(pointer.address())
    }

    pub async fn pointer_update(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
        counter: Option<u64>,
        cache_only: Option<CacheType>,
    ) -> Result<(), PointerError> {
        self.cache_pointer(owner, &target, counter, cache_only.clone());

        if !cache_only.is_some() {
            let command = Box::new(
                UpdatePointerCommand::new(self.client_harness.clone(), owner.clone(), target, counter)
            );
            self.send_update_command(command).await?;
        }
        Ok(())
    }

    fn cache_pointer(&self, owner: &SecretKey, target: &PointerTarget, counter: Option<u64>, cache_only: Option<CacheType>) -> Pointer {
        let pointer = Pointer::new(owner, counter.unwrap_or(0), target.clone());
        let ttl = if cache_only.is_some() { u64::MAX } else { self.ant_tp_config.cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(pointer.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer");
        info!("updating cache with pointer at address {}[{}] to target [{}] and TTL [{}]", POINTER_CACHE_KEY, pointer.address().to_hex(), target.to_hex(), ttl);
        if cache_only.is_some_and(|v| matches!(v, CacheType::Disk)) {
            self.hybrid_cache.insert(format!("{}{}", POINTER_CACHE_KEY, pointer.address().to_hex()), serialised_cache_item);
        } else {
            self.hybrid_cache.memory().insert(format!("{}{}", POINTER_CACHE_KEY, pointer.address().to_hex()), serialised_cache_item);
        }
        pointer
    }

    pub async fn pointer_get(&self, address: &PointerAddress) -> Result<Pointer, PointerError> {
        let cache_item = self.get_cache_item(address).await?;
        match cache_item.item {
            Some(_) => {
                info!("retrieved pointer for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    let command = Box::new(
                        GetPointerCommand::new(self.client_harness.clone(), self.hybrid_cache.clone(), address.clone(), self.ant_tp_config.cached_mutable_ttl)
                    );
                    self.send_get_command(command).await?;
                }
                Ok(cache_item.item.unwrap())
            }
            None => {
                info!("negative cache for pointer for [{}] from hybrid cache", address.to_hex());
                Err(PointerError::GetError(GetError::RecordNotFound(format!("Failed to retrieve pointer for [{}] from network", address.to_hex()))))
            }
        }
    }

    pub async fn pointer_update_ttl(&self,  address: &PointerAddress, ttl_override: u64) -> Result<Pointer, PointerError> {
        let cache_item = self.get_cache_item(address).await?;
        match cache_item.item {
            Some(_) => {
                let updated_cache_item = CacheItem::new(Some(cache_item.item.clone().unwrap()), ttl_override);
                match rmp_serde::to_vec(&updated_cache_item) {
                    Ok(serialized_cache_item) => {
                        self.hybrid_cache.insert(
                            format!("{}{}", POINTER_CACHE_KEY,  address.to_hex()),
                            serialized_cache_item
                        );
                    },
                    Err(e) => {
                        warn!("Failed to update TTL for pointer [{}] in hybrid cache: {}", address.to_hex(), e.to_string());
                    },
                }
                Ok(cache_item.item.unwrap())
            }
            None => {
                info!("negative cache for pointer for [{}] from hybrid cache", address.to_hex());
                Err(PointerError::GetError(GetError::RecordNotFound(format!("Failed to retrieve pointer for [{}] from network", address.to_hex()))))
            }
        }
    }

    async fn get_cache_item(&self, address: &PointerAddress) -> Result<CacheItem<Pointer>, PointerError> {
        let local_address = address.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        let cache_entry = self.hybrid_cache.get_ref().fetch(format!("{}{}", POINTER_CACHE_KEY, local_address.to_hex()), {
            let client = self.client_harness.get_ref().lock().await.get_client().await?;
            || async move {
                match client.pointer_get(&local_address).await {
                    Ok(pointer) => {
                        debug!("found pointer [{}] for address [{}]", hex::encode(pointer.target().to_hex()), local_address.to_hex());
                        let cache_item = CacheItem::new(Some(pointer.clone()), local_ant_tp_config.cached_mutable_ttl);
                        Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer"))
                    },
                    Err(e) => {
                        // store negative cache to avoid repeated lookups
                        debug!("failed to find pointer for address [{}]: {}", local_address.to_hex(), e);
                        let cache_item: CacheItem<Pointer> = CacheItem::new(None, local_ant_tp_config.cached_mutable_ttl * 10);
                        Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer"))
                    }
                }
            }
        }).await?;
        Ok(rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize pointer"))
    }

    pub async fn pointer_check_existence(&self, address: &PointerAddress) -> Result<bool, PointerError> {
        let local_address = address.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        let cache_entry = self.hybrid_cache.get_ref().fetch(format!("{}{}", POINTER_CHECK_CACHE_KEY, local_address.to_hex()), {
            let client = self.client_harness.get_ref().lock().await.get_client().await?;
            || async move {
                match client.pointer_check_existence(&local_address).await {
                    Ok(_) => {
                        debug!("pointer exists for address [{}]", local_address.to_hex());
                        let cache_item = CacheItem::new(Some(true), local_ant_tp_config.cached_mutable_ttl);
                        match rmp_serde::to_vec(&cache_item) {
                            Ok(cache_item) => Ok(cache_item),
                            Err(e) => Err(foyer::Error::other(format!("Failed to serialize pointer for [{}]: {}", local_address.to_hex(), e.to_string())))
                        }
                    },
                    Err(e) => {
                        // store negative cache to avoid repeated lookups
                        debug!("failed to find pointer exists for address [{}]: {}", local_address.to_hex(), e);
                        let cache_item: CacheItem<Pointer> = CacheItem::new(None, local_ant_tp_config.cached_mutable_ttl * 10);
                        Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize pointer"))
                    }
                }
            }
        }).await?;
        let cache_item: CacheItem<bool> = rmp_serde::from_slice(cache_entry.value())?;
        match cache_item.item {
            Some(_) => {
                info!("retrieved pointer check existence for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    let command = Box::new(
                        CheckPointerCommand::new(self.client_harness.clone(), self.hybrid_cache.clone(), address.clone(), self.ant_tp_config.cached_mutable_ttl)
                    );
                    self.send_check_command(command).await?;
                }
                Ok(cache_item.item.unwrap())
            }
            None => {
                info!("negative cache for pointer for [{}] from hybrid cache", address.to_hex());
                Err(PointerError::GetError(GetError::RecordNotFound(format!("Failed to pointer check existence for [{}] from network", local_address.to_hex()))))
            }
        }
    }
}