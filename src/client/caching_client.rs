use std::{env, fs};
use std::fs::File;
use std::io::{Read, Write};
use actix_web::web::Data;
use autonomi::{Client, Pointer, PointerAddress};
use autonomi::client::files::archive_public::{ArchiveAddress, PublicArchive};
use autonomi::client::GetError;
use autonomi::data::DataAddress;
use autonomi::pointer::{PointerError};
use autonomi::register::{RegisterAddress, RegisterError, RegisterValue};
use bytes::Bytes;
use log::{debug, info};
use xor_name::XorName;
use crate::{ClientCacheState};
use crate::client::cache_item::CacheItem;
use crate::config::anttp_config::AntTpConfig;
use crate::config::app_config::AppConfig;
use crate::service::archive_helper::ArchiveHelper;

#[derive(Clone)]
pub struct CachingClient {
    client: Client,
    cache_dir: String,
    ant_tp_config: AntTpConfig,
    client_cache_state: Data<ClientCacheState>,
}

impl CachingClient {

    pub fn new(client: Client, ant_tp_config: AntTpConfig, client_cache_state: Data<ClientCacheState>) -> Self {
        let cache_dir = env::temp_dir().to_str().unwrap().to_owned() + "/anttp/cache/";
        CachingClient::create_tmp_dir(cache_dir.clone());
        Self {
            client, cache_dir, ant_tp_config, client_cache_state,
        }
    }
    
    fn create_tmp_dir(cache_dir: String) {
        if !fs::exists(cache_dir.clone()).unwrap() {
            fs::create_dir_all(cache_dir.clone()).unwrap_or_default()
        }
    }

    /// Fetch an archive from the network
    pub async fn archive_get_public(&self, archive_address: ArchiveAddress) -> Result<PublicArchive, GetError> {
        let cached_data = self.read_file(archive_address).await;
        if !cached_data.is_empty() {
            debug!("getting cached public archive for [{}] from local storage", archive_address.to_hex());
            Ok(PublicArchive::from_bytes(cached_data)?)
        } else {
            debug!("getting uncached public archive for [{}] from network", archive_address.to_hex());
            let data = self.client.data_get_public(&archive_address).await?;
            self.write_file(archive_address, data.to_vec()).await;
            Ok(PublicArchive::from_bytes(data)?)
        }
    }

    pub async fn data_get_public(&self, addr: &DataAddress, ) -> Result<Bytes, GetError> {
        let cached_data = self.read_file(*addr).await;
        if !cached_data.is_empty() {
            debug!("getting cached data for [{}] from local storage", addr.to_hex());
            Ok(cached_data)
        } else {
            debug!("getting uncached data for [{}] from network", addr.to_hex());
            let data = self.client.data_get_public(addr).await?;
            self.write_file(*addr, data.to_vec()).await;
            Ok(data)
        }
    }

    pub async fn pointer_get(&self, address: &PointerAddress) -> Result<Pointer, PointerError> {
        if self.client_cache_state.get_ref().pointer_cache.lock().unwrap().contains_key(address)
            && !self.client_cache_state.get_ref().pointer_cache.lock().unwrap().get(address).unwrap().has_expired() {
            debug!("getting cached pointer for [{}] from memory", address.to_hex());
            match self.client_cache_state.get_ref().pointer_cache.lock().unwrap().get(address) {
                Some(cache_item) => {
                    debug!("getting cached pointer for [{}] from memory", address.to_hex());
                    match cache_item.item.clone() {
                        Some(pointer) => Ok(pointer),
                        None => Err(PointerError::Serialization)
                    }
                }
                None => Err(PointerError::Serialization)
            }
        } else {
            self.pointer_get_uncached(address).await
        }
    }

    async fn pointer_get_uncached(&self, address: &PointerAddress) -> Result<Pointer, PointerError> {
        debug!("getting uncached pointer for [{}] from network", address.to_hex());
        match self.client.pointer_get(address).await {
            Ok(pointer) => {
                debug!("found pointer [{}] for address [{}]", hex::encode(pointer.target().to_hex()), address.to_hex());
                self.client_cache_state.get_ref().pointer_cache.lock().unwrap().insert(address.clone(), CacheItem::new(Some(pointer.clone()), self.ant_tp_config.clone().cached_mutable_ttl));
                Ok(pointer)
            }
            Err(_) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no pointer for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().pointer_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(PointerError::Serialization)
            }
        }
    }

    pub async fn register_get(&self, address: &RegisterAddress) -> Result<RegisterValue, RegisterError> {
        if self.client_cache_state.get_ref().register_cache.lock().unwrap().contains_key(address)
            && !self.client_cache_state.get_ref().register_cache.lock().unwrap().get(address).unwrap().has_expired() {
            debug!("getting cached register for [{}] from memory", address.to_hex());
            match self.client_cache_state.get_ref().register_cache.lock().unwrap().get(address) {
                Some(cache_item) => {
                    debug!("getting cached register for [{}] from memory", address.to_hex());
                    match cache_item.item.clone() {
                        Some(register_value) => Ok(register_value),
                        None => Err(RegisterError::PointerError(PointerError::Serialization))
                    }
                }
                None => Err(RegisterError::PointerError(PointerError::Serialization))
            }
        } else {
            self.register_get_uncached(address).await
        }
    }

    async fn register_get_uncached(&self, address: &RegisterAddress) -> Result<RegisterValue, RegisterError> {
        debug!("getting uncached register for [{}] from network", address.to_hex());
        match self.client.register_get(address).await {
            Ok(register_value) => {
                debug!("found register value [{}] for address [{}]", hex::encode(register_value), address.to_hex());
                self.client_cache_state.get_ref().register_cache.lock().unwrap().insert(address.clone(), CacheItem::new(Some(register_value.clone()), self.ant_tp_config.clone().cached_mutable_ttl));
                Ok(register_value)
            },
            Err(_) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no register value for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().register_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(RegisterError::PointerError(PointerError::Serialization))
            }
        }
    }

    pub async fn write_file(&self, archive_address: ArchiveAddress, data: Vec<u8>) {
        let path_string = self.cache_dir.clone() + format!("{:x}", archive_address.xorname()).as_str();
        let mut file = File::create(path_string).unwrap();
        file.write_all(data.as_slice()).unwrap();
    }

    pub async fn read_file(&self, archive_address: ArchiveAddress) -> Bytes {
        let path_string = self.cache_dir.clone() + format!("{:x}", archive_address.xorname()).as_str();
        match File::open(path_string) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).unwrap();
                Bytes::from(contents.clone())
            },
            Err(_) => {
                Bytes::from("")
            }
        }
    }

    pub async fn config_get_public(&self, archive: PublicArchive, archive_address_xorname: XorName) -> AppConfig {
        let path_str = "app-conf.json";
        let mut path_parts = Vec::<String>::new();
        path_parts.push("ignore".to_string());
        path_parts.push(path_str.to_string());
        match ArchiveHelper::new(archive, self.ant_tp_config.clone()).resolve_data_addr(path_parts) {
            Ok(data_address) => {
                info!("Downloading app-config [{}] with addr [{}] from archive [{}]", path_str, format!("{:x}", data_address.xorname()), format!("{:x}", archive_address_xorname));
                match self.data_get_public(&data_address).await {
                    Ok(data) => {
                        let json = String::from_utf8(data.to_vec()).unwrap_or(String::new());
                        debug!("json [{}]", json);
                        let config: AppConfig = serde_json::from_str(&json.as_str())
                            .unwrap_or(AppConfig::default());
                        config
                    }
                    Err(_e) => AppConfig::default()
                }
            }
            Err(_e) => AppConfig::default()
        }
    }
}