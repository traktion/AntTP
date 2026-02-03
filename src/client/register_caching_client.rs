use autonomi::client::payment::PaymentOption;
use autonomi::register::{RegisterAddress, RegisterHistory, RegisterValue};
use autonomi::SecretKey;
use log::{debug, info};
use mockall_double::double;
use crate::client::cache_item::CacheItem;
#[double]
use crate::client::CachingClient;
use crate::client::REGISTER_CACHE_KEY;
use crate::client::command::register::create_register_command::CreateRegisterCommand;
use crate::client::command::register::get_register_command::GetRegisterCommand;
use crate::client::command::register::update_register_command::UpdateRegisterCommand;
use crate::controller::StoreType;
use crate::error::register_error::RegisterError;

#[derive(Debug, Clone)]
pub struct RegisterCachingClient {
    caching_client: CachingClient,
}

#[mockall::automock]
impl RegisterCachingClient {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn register_create(
        &self,
        owner: &SecretKey,
        register_value: RegisterValue,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<RegisterAddress, RegisterError> {
        let register_address = self.cache_register(owner, &register_value, store_type.clone());

        if store_type == StoreType::Network {
            let command = Box::new(
                CreateRegisterCommand::new(self.caching_client.get_client_harness().clone(), owner.clone(), register_value, payment_option)
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(register_address)
    }

    pub async fn register_update(
        &self,
        owner: &SecretKey,
        register_value: RegisterValue,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<(), RegisterError> {
        self.cache_register(owner, &register_value, store_type.clone());

        if store_type == StoreType::Network {
            let command = Box::new(
                UpdateRegisterCommand::new(self.caching_client.get_client_harness().clone(), owner.clone(), register_value, payment_option)
            );
            self.caching_client.send_update_command(command).await?;
        }
        Ok(())
    }

    fn cache_register(&self, owner: &SecretKey, register_value: &RegisterValue, store_type: StoreType) -> RegisterAddress {
        let register_address = RegisterAddress::new(owner.public_key());
        let ttl = if store_type != StoreType::Network { u64::MAX } else { self.caching_client.get_ant_tp_config().cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(register_value.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize register");
        info!("updating cache with register at address {}[{}] to value [{:?}] and TTL [{}]", REGISTER_CACHE_KEY, register_address.to_hex(), register_value, ttl);
        if store_type == StoreType::Disk {
            self.caching_client.get_hybrid_cache().insert(format!("{}{}", REGISTER_CACHE_KEY, register_address.to_hex()), serialised_cache_item);
        } else {
            self.caching_client.get_hybrid_cache().memory().insert(format!("{}{}", REGISTER_CACHE_KEY, register_address.to_hex()), serialised_cache_item);
        }
        register_address
    }

    pub async fn register_get(&self, address: &RegisterAddress) -> Result<RegisterValue, RegisterError> {
        let local_address = address.clone();
        let local_ant_tp_config = self.caching_client.get_ant_tp_config().clone();
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().fetch(format!("{}{}", REGISTER_CACHE_KEY, local_address.to_hex()), {
            let client = self.caching_client.get_client_harness().get_ref().lock().await.get_client().await?;
            || async move {
                match client.register_get(&local_address).await {
                    Ok(register_value) => {
                        debug!("found register value [{}] for address [{}] from network", hex::encode(register_value.clone()), local_address.to_hex());
                        let cache_item = CacheItem::new(Some(register_value.clone()), local_ant_tp_config.cached_mutable_ttl);
                        match rmp_serde::to_vec(&cache_item) {
                            Ok(cache_item) => Ok(cache_item),
                            Err(e) => Err(foyer::Error::other(format!("Failed to serialize register for [{}]: {}", local_address.to_hex(), e.to_string())))
                        }
                    }
                    Err(e) => Err(foyer::Error::other(format!("Failed to retrieve register for [{}] from network: {}", local_address.to_hex(), e.to_string())))
                }
            }
        }).await?;
        let cache_item: CacheItem<RegisterValue> = rmp_serde::from_slice(cache_entry.value())?;
        info!("retrieved register for [{}] from hybrid cache", address.to_hex());
        if cache_item.has_expired() {
            let command = Box::new(
                GetRegisterCommand::new(self.caching_client.get_client_harness().clone(), self.caching_client.get_hybrid_cache().clone(), address.clone(), self.caching_client.get_ant_tp_config().cached_mutable_ttl)
            );
            self.caching_client.send_get_command(command).await?;
        }
        Ok(cache_item.item.unwrap())
    }

    pub async fn register_history(&self, addr: &RegisterAddress) -> Result<RegisterHistory, RegisterError> {
        Ok(self.caching_client.get_client_harness().get_ref().lock().await.get_client().await?.register_history(addr))
    }
}