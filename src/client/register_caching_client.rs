use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::register::{RegisterAddress, RegisterError, RegisterHistory, RegisterValue};
use autonomi::SecretKey;
use log::{debug, info};
use crate::client::cache_item::CacheItem;
use crate::client::CachingClient;
use crate::client::command::register::create_register_command::CreateRegisterCommand;
use crate::client::command::register::get_register_command::GetRegisterCommand;
use crate::client::command::register::update_register_command::UpdateRegisterCommand;
use crate::controller::CacheType;

impl CachingClient {

    pub async fn register_create(
        &self,
        owner: &SecretKey,
        register_value: RegisterValue,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(AttoTokens, RegisterAddress), RegisterError> {
        let register_address = self.cache_register(owner, &register_value, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(CreateRegisterCommand::new(self.client_harness.clone(), owner.clone(), register_value, payment_option))
            ).await.unwrap();
        }
        Ok((AttoTokens::zero(), register_address))
    }

    pub async fn register_update(
        &self,
        owner: &SecretKey,
        register_value: RegisterValue,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<AttoTokens, RegisterError> {
        self.cache_register(owner, &register_value, cache_only.clone());

        if !cache_only.is_some() {
            self.command_executor.send(
                Box::new(UpdateRegisterCommand::new(self.client_harness.clone(), owner.clone(), register_value, payment_option))
            ).await.unwrap();
        }
        Ok(AttoTokens::zero())
    }

    fn cache_register(&self, owner: &SecretKey, register_value: &RegisterValue, cache_only: Option<CacheType>) -> RegisterAddress {
        let register_address = RegisterAddress::new(owner.public_key());
        let ttl = if cache_only.is_some() { u64::MAX } else { self.ant_tp_config.cached_mutable_ttl };
        let cache_item = CacheItem::new(Some(register_value.clone()), ttl);
        let serialised_cache_item = rmp_serde::to_vec(&cache_item).expect("Failed to serialize register");
        info!("updating cache with register at address rg[{}] to value [{:?}] and TTL [{}]", register_address.to_hex(), register_value, ttl);
        if cache_only.is_some_and(|v| matches!(v, CacheType::Disk)) {
            self.hybrid_cache.insert(format!("rg{}", register_address.to_hex()), serialised_cache_item);
        } else {
            self.hybrid_cache.memory().insert(format!("rg{}", register_address.to_hex()), serialised_cache_item);
        }
        register_address
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
                                debug!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
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
                    self.command_executor.send(
                        Box::new(GetRegisterCommand::new(self.client_harness.clone(), self.hybrid_cache.clone(), address.clone(), self.ant_tp_config.cached_mutable_ttl))
                    ).await.unwrap();
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