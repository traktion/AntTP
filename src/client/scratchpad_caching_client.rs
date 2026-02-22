use autonomi::client::payment::PaymentOption;
use autonomi::{Scratchpad, ScratchpadAddress, SecretKey};
use bytes::Bytes;
use log::{debug, info};
use mockall_double::double;
use crate::client::cache_item::CacheItem;
#[double]
use crate::client::CachingClient;
use crate::client::SCRATCHPAD_CACHE_KEY;
use crate::client::command::scratchpad::create_private_scratchpad_command::CreatePrivateScratchpadCommand;
use crate::client::command::scratchpad::create_public_scratchpad_command::CreatePublicScratchpadCommand;
use crate::client::command::scratchpad::get_scratchpad_command::GetScratchpadCommand;
use crate::client::command::scratchpad::update_private_scratchpad_command::UpdatePrivateScratchpadCommand;
use crate::client::command::scratchpad::update_public_scratchpad_command::UpdatePublicScratchpadCommand;
use crate::controller::StoreType;
use crate::error::scratchpad_error::ScratchpadError;

use mockall::mock;

#[derive(Debug, Clone)]
pub struct ScratchpadCachingClient {
    caching_client: CachingClient,
}

mock! {
    #[derive(Debug)]
    pub ScratchpadCachingClient {
        pub fn new(caching_client: CachingClient) -> Self;
        pub async fn scratchpad_create(
            &self,
            owner: &SecretKey,
            content_type: u64,
            data: &Bytes,
            payment_option: PaymentOption,
            store_type: StoreType,
        ) -> Result<ScratchpadAddress, ScratchpadError>;
        pub async fn scratchpad_update(
            &self,
            owner: &SecretKey,
            content_type: u64,
            data: &Bytes,
            store_type: StoreType,
        ) -> Result<(), ScratchpadError>;
        pub async fn scratchpad_create_public(
            &self,
            owner: &SecretKey,
            content_type: u64,
            data: &Bytes,
            payment_option: PaymentOption,
            store_type: StoreType,
        ) -> Result<ScratchpadAddress, ScratchpadError>;
        pub async fn scratchpad_update_public(
            &self,
            owner: &SecretKey,
            content_type: u64,
            data: &Bytes,
            payment_option: PaymentOption,
            store_type: StoreType,
        ) -> Result<(), ScratchpadError>;
        pub async fn scratchpad_get(&self, address: &ScratchpadAddress) -> Result<autonomi::Scratchpad, ScratchpadError>;
    }
    impl Clone for ScratchpadCachingClient {
        fn clone(&self) -> Self;
    }
}

impl ScratchpadCachingClient {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn scratchpad_create(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<ScratchpadAddress, ScratchpadError> {
        let scratchpad_address = self.cache_scratchpad(owner, content_type, data, true, store_type.clone());

        if store_type == StoreType::Network {
            let command = Box::new(
                CreatePrivateScratchpadCommand::new(self.caching_client.get_client_harness().clone(), owner.clone(), content_type, data.clone(), payment_option)
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(scratchpad_address)
    }

    pub async fn scratchpad_update(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        store_type: StoreType,
    ) -> Result<(), ScratchpadError> {
        self.cache_scratchpad(owner, content_type, data, true, store_type.clone());

        if store_type == StoreType::Network {
            let command = Box::new(
                UpdatePrivateScratchpadCommand::new(self.caching_client.get_client_harness().clone(), owner.clone(), content_type, data.clone())
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(())
    }

    pub async fn scratchpad_create_public(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<ScratchpadAddress, ScratchpadError> {
        let scratchpad_address = self.cache_scratchpad(owner, content_type, data, false, store_type.clone());

        if store_type == StoreType::Network {
            let command = Box::new(
                CreatePublicScratchpadCommand::new(self.caching_client.get_client_harness().clone(), owner.clone(), content_type, data.clone(), payment_option)
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(scratchpad_address)
    }

    pub async fn scratchpad_update_public(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<(), ScratchpadError> {
        self.cache_scratchpad(owner, content_type, data, false, store_type.clone());

        if store_type == StoreType::Network {
            let command = Box::new(
                UpdatePublicScratchpadCommand::new(self.caching_client.get_client_harness().clone(), owner.clone(), content_type, data.clone(), payment_option)
            );
            self.caching_client.send_update_command(command).await?;
        }
        Ok(())
    }

    fn cache_scratchpad(&self, owner: &SecretKey, content_type: u64, data: &Bytes, is_encrypted: bool, store_type: StoreType) -> ScratchpadAddress {
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

        let ttl = if store_type != StoreType::Network { u64::MAX } else { self.caching_client.get_ant_tp_config().cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(scratchpad.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize scratchpad");
        info!("updating cache with scratchpad at address {}[{}] to value [{:?}] and TTL [{}]", SCRATCHPAD_CACHE_KEY, scratchpad_address.to_hex(), scratchpad, ttl);
        if store_type == StoreType::Disk {
            self.caching_client.get_hybrid_cache().insert(format!("{}{}", SCRATCHPAD_CACHE_KEY, scratchpad_address.to_hex()), serialised_cache_item);
        } else {
            self.caching_client.get_hybrid_cache().memory().insert(format!("{}{}", SCRATCHPAD_CACHE_KEY, scratchpad_address.to_hex()), serialised_cache_item);
        }
        scratchpad_address
    }

    pub async fn scratchpad_get(&self, address: &ScratchpadAddress) -> Result<Scratchpad, ScratchpadError> {
        let local_address = address.clone();
        let local_ant_tp_config = self.caching_client.get_ant_tp_config().clone();
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().fetch(format!("{}{}", SCRATCHPAD_CACHE_KEY, local_address.to_hex()), {
            let client = self.caching_client.get_client_harness().get_ref().lock().await.get_client().await?;
            || async move {
                match client.scratchpad_get(&local_address).await {
                    Ok(scratchpad) => {
                        debug!("found scratchpad for address [{}]", local_address.to_hex());
                        let cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                        match rmp_serde::to_vec(&cache_item) {
                            Ok(cache_item) => Ok(cache_item),
                            Err(e) => Err(foyer::Error::other(format!("Failed to serialize scratchpad for [{}]: {}", local_address.to_hex(), e.to_string())))
                        }
                    }
                    Err(e) => Err(foyer::Error::other(format!("Failed to retrieve scratchpad for [{}] from network: {}", local_address.to_hex(), e.to_string())))
                }
            }
        }).await?;
        let cache_item: CacheItem<Scratchpad> = rmp_serde::from_slice(cache_entry.value())?;
        info!("retrieved scratchpad for [{}] from hybrid cache", address.to_hex());
        if cache_item.has_expired() {
            let command = Box::new(
                GetScratchpadCommand::new(self.caching_client.get_client_harness().clone(), self.caching_client.get_hybrid_cache().clone(), address.clone(), self.caching_client.get_ant_tp_config().cached_mutable_ttl)
            );
            self.caching_client.send_get_command(command).await?;
        }
        Ok(cache_item.item.unwrap())
    }
}