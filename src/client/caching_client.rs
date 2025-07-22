use std::{env, fs};
use std::fs::File;
use std::io::{Read, Write};
use actix_web::web::Data;
use ant_evm::AttoTokens;
use autonomi::{Chunk, ChunkAddress, Client, GraphEntry, GraphEntryAddress, Pointer, PointerAddress, ScratchpadAddress, SecretKey};
use autonomi::client::files::archive_public::{ArchiveAddress, PublicArchive};
use autonomi::client::{GetError, PutError};
use autonomi::client::payment::PaymentOption;
use autonomi::data::DataAddress;
use autonomi::graph::GraphError;
use autonomi::pointer::{PointerError, PointerTarget};
use autonomi::register::{RegisterAddress, RegisterError, RegisterHistory, RegisterValue};
use autonomi::scratchpad::{Scratchpad, ScratchpadError};
use bytes::Bytes;
use log::{debug, info};
use rmp_serde::decode;
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
        let cache_dir = if ant_tp_config.map_cache_directory.is_empty() {
            env::temp_dir().to_str().unwrap().to_owned() + "/anttp/cache/"
        } else {
            ant_tp_config.map_cache_directory.clone()
        };
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
    pub async fn archive_get_public(&self, archive_address: ArchiveAddress) -> Result<PublicArchive, decode::Error> {
        let cached_data = self.read_file(archive_address).await;
        if !cached_data.is_empty() {
            debug!("getting cached public archive for [{}] from local storage", archive_address.to_hex());
            Ok(PublicArchive::from_bytes(cached_data)?)
        } else {
            debug!("getting uncached public archive for [{}] from network", archive_address.to_hex());
            match self.client.data_get_public(&archive_address).await {
                Ok(data) => match PublicArchive::from_bytes(data.clone()) {
                    Ok(public_archive) => {
                        self.write_file(archive_address, data.to_vec()).await;
                        Ok(public_archive)
                    },
                    Err(err) => Err(err)
                },
                Err(err) => Err(decode::Error::Uncategorized(format!("Failed to retrieve public data: {:?}", err)))
            }
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

    pub async fn pointer_create(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, PointerAddress), PointerError> {
        let client_clone = self.client.clone();
        let owner_clone = owner.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("creating pointer async");
            client_clone.pointer_create(&owner_clone, target, payment_option).await
        });
        let address = PointerAddress::new(owner.public_key());
        Ok((AttoTokens::zero(), address))
    }

    pub async fn pointer_update(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
    ) -> Result<(), PointerError> {
        let client_clone = self.client.clone();
        let owner_clone = owner.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("updating pointer async");
            client_clone.pointer_update(&owner_clone, target).await
        });
        Ok(())
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
            Err(e) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no pointer for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().pointer_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(e)
            }
        }
    }

    pub async fn register_create(
        &self,
        owner: &SecretKey,
        initial_value: RegisterValue,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, RegisterAddress), RegisterError> {
        let client_clone = self.client.clone();
        let owner_clone = owner.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("creating register async");
            client_clone.register_create(&owner_clone, initial_value, payment_option).await
        });
        Ok((AttoTokens::zero(), RegisterAddress::new(owner.clone().public_key())))
    }

    pub async fn register_update(
        &self,
        owner: &SecretKey,
        new_value: RegisterValue,
        payment_option: PaymentOption,
    ) -> Result<AttoTokens, RegisterError> {
        let client_clone = self.client.clone();
        let owner_clone = owner.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("updating register async");
            client_clone.register_update(&owner_clone, new_value, payment_option).await
        });
        Ok(AttoTokens::zero())
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
            Err(e) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no register value for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().register_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(e)
            }
        }
    }

    pub fn register_history(&self, addr: &RegisterAddress) -> RegisterHistory {
        self.client.register_history(addr)
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

    pub async fn scratchpad_create(
        &self,
        owner: &SecretKey,
        content_type: u64,
        initial_data: &Bytes,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, ScratchpadAddress), ScratchpadError> {
        let client_clone = self.client.clone();
        let owner_clone = owner.clone();
        let initial_data_clone = initial_data.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("creating scratchpad async");
            client_clone.scratchpad_create(&owner_clone, content_type, &initial_data_clone, payment_option).await
        });
        let address = ScratchpadAddress::new(owner.public_key());
        Ok((AttoTokens::zero(), address))
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
        let client_clone = self.client.clone();
        tokio::spawn(async move {
            debug!("creating scratchpad async");
            client_clone.scratchpad_put(scratchpad, payment_option).await
        });
        Ok((AttoTokens::zero(), address))
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
        let client_clone = self.client.clone();
        tokio::spawn(async move {
            debug!("creating scratchpad async");
            client_clone.scratchpad_put(scratchpad, payment_option).await
        });
        Ok(())
    }

    pub async fn scratchpad_check_existance(
        &self,
        address: &ScratchpadAddress,
    ) -> Result<bool, ScratchpadError> {
        self.client.scratchpad_check_existence(address).await
    }

    pub async fn scratchpad_update(
        &self,
        owner: &SecretKey,
        content_type: u64,
        data: &Bytes,
    ) -> Result<(), ScratchpadError> {
        let client_clone = self.client.clone();
        let owner_clone = owner.clone();
        let data_clone = data.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("updating scratchpad async");
            client_clone.scratchpad_update(&owner_clone, content_type, &data_clone).await
        });
        Ok(())
    }

    pub async fn scratchpad_get(&self, address: &ScratchpadAddress) -> Result<Scratchpad, ScratchpadError> {
        if self.client_cache_state.get_ref().scratchpad_cache.lock().unwrap().contains_key(address)
            && !self.client_cache_state.get_ref().scratchpad_cache.lock().unwrap().get(address).unwrap().has_expired() {
            debug!("getting cached scratchpad for [{}] from memory", address.to_hex());
            match self.client_cache_state.get_ref().scratchpad_cache.lock().unwrap().get(address) {
                Some(cache_item) => {
                    debug!("getting cached scratchpad for [{}] from memory", address.to_hex());
                    match cache_item.item.clone() {
                        Some(scratchpad) => Ok(scratchpad),
                        None => Err(ScratchpadError::Serialization)
                    }
                }
                None => Err(ScratchpadError::Serialization)
            }
        } else {
            self.scratchpad_get_uncached(address).await
        }
    }

    async fn scratchpad_get_uncached(&self, address: &ScratchpadAddress) -> Result<Scratchpad, ScratchpadError> {
        debug!("getting uncached scratchpad for [{}] from network", address.to_hex());
        match self.client.scratchpad_get(address).await {
            Ok(scratchpad) => {
                debug!("found scratchpad for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().scratchpad_cache.lock().unwrap().insert(address.clone(), CacheItem::new(Some(scratchpad.clone()), self.ant_tp_config.clone().cached_mutable_ttl));
                Ok(scratchpad)
            }
            Err(e) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no scratchpad for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().scratchpad_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(e)
            }
        }
    }

    pub async fn chunk_put(
        &self,
        chunk: &Chunk,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, ChunkAddress), PutError> {
        let client_clone = self.client.clone();
        let chunk_clone = chunk.clone();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("creating chunk async");
            client_clone.chunk_put(&chunk_clone, payment_option).await
        });
        Ok((AttoTokens::zero(), chunk.address))
    }

    pub async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, GetError> {
        if self.client_cache_state.get_ref().chunk_cache.lock().unwrap().contains_key(address)
            && !self.client_cache_state.get_ref().chunk_cache.lock().unwrap().get(address).unwrap().has_expired() {
            debug!("getting cached chunk for [{}] from memory", address.to_hex());
            match self.client_cache_state.get_ref().chunk_cache.lock().unwrap().get(address) {
                Some(cache_item) => {
                    debug!("getting cached chunk for [{}] from memory", address.to_hex());
                    match cache_item.item.clone() {
                        Some(chunk) => Ok(chunk),
                        None => Err(GetError::InvalidDataMap(decode::Error::Uncategorized("Failed to find chunk in cache".to_string())))
                    }
                }
                None => Err(GetError::InvalidDataMap(decode::Error::Uncategorized("Failed to find chunk in cache".to_string())))
            }
        } else {
            self.chunk_get_uncached(address).await
        }
    }

    async fn chunk_get_uncached(&self, address: &ChunkAddress) -> Result<Chunk, GetError> {
        debug!("getting uncached chunk for [{}] from network", address.to_hex());
        match self.client.chunk_get(address).await {
            Ok(chunk) => {
                debug!("found chunk for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().chunk_cache.lock().unwrap().insert(address.clone(), CacheItem::new(Some(chunk.clone()), self.ant_tp_config.clone().cached_mutable_ttl));
                Ok(chunk)
            }
            Err(e) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no chunk for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().chunk_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(e)
            }
        }
    }

    pub async fn graph_entry_put(
        &self,
        entry: GraphEntry,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, GraphEntryAddress), GraphError> {
        let client_clone = self.client.clone();
        let address = entry.address();
        // todo: move to job processor
        tokio::spawn(async move {
            debug!("creating graph entry async");
            client_clone.graph_entry_put(entry, payment_option).await
        });
        Ok((AttoTokens::zero(), address))
    }

    pub async fn graph_entry_get(
        &self,
        address: &GraphEntryAddress,
    ) -> Result<GraphEntry, GraphError> {
        if self.client_cache_state.get_ref().graph_entry_cache.lock().unwrap().contains_key(address)
            && !self.client_cache_state.get_ref().graph_entry_cache.lock().unwrap().get(address).unwrap().has_expired() {
            debug!("getting cached graph for [{}] from memory", address.to_hex());
            match self.client_cache_state.get_ref().graph_entry_cache.lock().unwrap().get(address) {
                Some(cache_item) => {
                    debug!("getting cached graph for [{}] from memory", address.to_hex());
                    match cache_item.item.clone() {
                        Some(graph) => Ok(graph),
                        None => Err(GraphError::Serialization("Failed to fetch item from cache".to_string()))
                    }
                }
                None => Err(GraphError::Serialization("Failed to find item in cache".to_string()))
            }
        } else {
            self.graph_entry_get_uncached(address).await
        }
    }

    pub async fn graph_entry_get_uncached(
        &self,
        address: &GraphEntryAddress,
    ) -> Result<GraphEntry, GraphError> {
        debug!("getting uncached graph entry for [{}] from network", address.to_hex());
        match self.client.graph_entry_get(address).await {
            Ok(graph_entry) => {
                debug!("found graph entry for address [{}]",  address.to_hex());
                self.client_cache_state.get_ref().graph_entry_cache.lock().unwrap().insert(address.clone(), CacheItem::new(Some(graph_entry.clone()), self.ant_tp_config.clone().cached_mutable_ttl));
                Ok(graph_entry)
            }
            Err(e) => {
                // cache mismatches to avoid repeated lookup
                debug!("found no graph entry for address [{}]", address.to_hex());
                self.client_cache_state.get_ref().graph_entry_cache.lock().unwrap().insert(address.clone(), CacheItem::new(None, self.ant_tp_config.clone().cached_mutable_ttl));
                Err(e)
            }
        }
    }
}