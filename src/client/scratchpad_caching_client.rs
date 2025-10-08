use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::{Scratchpad, ScratchpadAddress, SecretKey};
use autonomi::scratchpad::ScratchpadError;
use bytes::Bytes;
use log::{debug, info};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;
use crate::client::command::scratchpad::create_private_scratchpad_command::CreatePrivateScratchpadCommand;
use crate::client::command::scratchpad::create_public_scratchpad_command::CreatePublicScratchpadCommand;
use crate::client::command::scratchpad::get_scratchpad_command::GetScratchpadCommand;
use crate::client::command::scratchpad::update_private_scratchpad_command::UpdatePrivateScratchpadCommand;
use crate::client::command::scratchpad::update_public_scratchpad_command::UpdatePublicScratchpadCommand;
use crate::controller::CacheType;

impl CachingClient {
    
    pub async fn scratchpad_create(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(AttoTokens, ScratchpadAddress), ScratchpadError> {
        let scratchpad_address = self.cache_scratchpad(owner, content_type, data, true, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(CreatePrivateScratchpadCommand::new(self.client_harness.clone(), owner.clone(), content_type, data.clone(), payment_option))
            ).await.unwrap();
        }
        Ok((AttoTokens::zero(), scratchpad_address))
    }

    pub async fn scratchpad_update(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        cache_only: Option<CacheType>,
    ) -> Result<(), ScratchpadError> {
        self.cache_scratchpad(owner, content_type, data, true, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(UpdatePrivateScratchpadCommand::new(self.client_harness.clone(), owner.clone(), content_type, data.clone()))
            ).await.unwrap();
        }
        Ok(())
    }

    pub async fn scratchpad_create_public(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(AttoTokens, ScratchpadAddress), ScratchpadError> {
        let scratchpad_address = self.cache_scratchpad(owner, content_type, data, false, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(CreatePublicScratchpadCommand::new(self.client_harness.clone(), owner.clone(), content_type, data.clone(), payment_option))
            ).await.unwrap();
        }
        Ok((AttoTokens::zero(), scratchpad_address))
    }

    pub async fn scratchpad_update_public(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(), ScratchpadError> {
        self.cache_scratchpad(owner, content_type, data, false, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(UpdatePublicScratchpadCommand::new(self.client_harness.clone(), owner.clone(), content_type, data.clone(), payment_option))
            ).await.unwrap();
        }
        Ok(())
    }

    fn cache_scratchpad(&self, owner: &SecretKey, content_type: u64, data: &Bytes, is_encrypted: bool, cache_only: Option<CacheType>) -> ScratchpadAddress {
        let scratchpad_address = ScratchpadAddress::new(owner.public_key());

        let scratchpad = if is_encrypted {
            Scratchpad::new(owner, content_type, &data.clone(), 0)
        } else {
            let signature = owner.sign(Scratchpad::bytes_for_signature(
                scratchpad_address,
                content_type,
                &data.clone(),
                0,
            ));
            Scratchpad::new_with_signature(owner.public_key(), content_type, data.clone(), 0, signature)
        };

        let ttl = if cache_only.is_some() { u64::MAX } else { self.ant_tp_config.cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(scratchpad.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize register");
        info!("updating cache with register at address sg[{}] to value [{:?}] and TTL [{}]", scratchpad_address.to_hex(), scratchpad, ttl);
        if cache_only.is_some_and(|v| matches!(v, CacheType::Disk)) {
            self.hybrid_cache.insert(format!("sg{}", scratchpad_address.to_hex()), serialised_cache_item);
        } else {
            self.hybrid_cache.memory().insert(format!("sg{}", scratchpad_address.to_hex()), serialised_cache_item);
        }
        scratchpad_address
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
                                debug!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
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
                    self.command_executor.send(
                        Box::new(GetScratchpadCommand::new(self.client_harness.clone(), self.hybrid_cache.clone(), address.clone(), self.ant_tp_config.cached_mutable_ttl))
                    ).await.unwrap();
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(ScratchpadError::Serialization),
        }
    }
}