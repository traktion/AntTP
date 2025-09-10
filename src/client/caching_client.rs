use std::{fs};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use actix_web::Error;
use actix_web::error::ErrorInternalServerError;
use actix_web::web::Data;
use ant_evm::AttoTokens;
use async_job::{Job, Schedule};
use async_trait::async_trait;
use autonomi::{Chunk, ChunkAddress, GraphEntry, GraphEntryAddress, Pointer, PointerAddress, ScratchpadAddress, SecretKey};
use autonomi::client::files::archive_public::{ArchiveAddress, PublicArchive};
use autonomi::client::{GetError, PutError};
use autonomi::client::payment::PaymentOption;
use autonomi::data::DataAddress;
use autonomi::files::UploadError;
use autonomi::graph::GraphError;
use autonomi::pointer::{PointerError, PointerTarget};
use autonomi::register::{RegisterAddress, RegisterError, RegisterHistory, RegisterValue};
use autonomi::scratchpad::{Scratchpad, ScratchpadError};
use chunk_streamer::chunk_streamer::{ChunkGetter, ChunkStreamer};
use foyer::HybridCache;
use log::{debug, error, info, warn};
use rmp_serde::decode;
use crate::client::cache_item::CacheItem;
use crate::config::anttp_config::AntTpConfig;
use bytes::{BufMut, Bytes, BytesMut};
use futures_util::StreamExt;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::model::archive::Archive;

#[derive(Clone)]
pub struct CachingClient {
    client_harness: Data<Mutex<ClientHarness>>,
    cache_dir: String,
    ant_tp_config: AntTpConfig,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
}

pub const ARCHIVE_TAR_IDX_BYTES: &[u8] = "\0archive.tar.idx\0".as_bytes();

#[async_trait]
impl Job for CachingClient {
    fn schedule(&self) -> Option<Schedule> {
        Some("1/10 * * * * *".parse().unwrap())
    }
    async fn handle(&mut self) {
        self.client_harness.get_ref().lock().await.try_sleep();
    }
}

#[async_trait]
impl ChunkGetter for CachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, GetError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(local_address.to_hex(), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
            match maybe_local_client {
                Some(local_client) => {
                    match local_client.chunk_get(&local_address).await {
                        Ok(chunk) => {
                            info!("retrieved chunk for [{}] from network - storing in hybrid cache", local_address.to_hex());
                            info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                            Ok(Vec::from(chunk.value))
                        }
                        Err(err) => {
                            error!("Failed to retrieve chunk for [{}] from network {:?}", local_address.to_hex(), err);
                            Err(foyer::Error::other(format!("Failed to retrieve chunk for [{}] from network {:?}", local_address.to_hex(), err)))
                        }
                    }
                },
                None => {
                    error!("Failed to retrieve chunk for [{}] as offline network", local_address.to_hex());
                    Err(foyer::Error::other(format!("Failed to retrieve chunk for [{}] from offline network", local_address.to_hex())))
                }
            }
        }}).await {
            Ok(cache_entry) => {
                info!("retrieved chunk for [{}] from hybrid cache", address.to_hex());
                Ok(Chunk::new(Bytes::from(cache_entry.value().to_vec())))
            },
            Err(_) => Err(GetError::RecordNotFound)
        }
    }
}

impl CachingClient {

    pub fn new(client_harness: Data<Mutex<ClientHarness>>, ant_tp_config: AntTpConfig, hybrid_cache: Data<HybridCache<String, Vec<u8>>>) -> Self {
        let cache_dir = ant_tp_config.clone().map_cache_directory;
        CachingClient::create_tmp_dir(cache_dir.clone());

        Self {
            client_harness, cache_dir, ant_tp_config, hybrid_cache,
        }
    }

    fn create_tmp_dir(cache_dir: String) {
        if !fs::exists(cache_dir.clone()).unwrap() {
            fs::create_dir_all(cache_dir.clone()).unwrap_or_default()
        }
    }

    pub async fn archive_get(&self, addr: ArchiveAddress) -> Result<Archive, decode::Error> {
        // todo: could remove caching of sub-calls, unless called directly elsewhere?
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(format!("ar{}", local_address.to_hex()), || async move {
            // todo: enable join agani
            //let (public_archive, tarchive) = join!(local_caching_client.data_get_public(&addr), local_caching_client.get_archive_from_tar(&addr));
            let public_archive = local_caching_client.data_get_public(&addr).await;
            let tarchive = local_caching_client.get_archive_from_tar(&addr).await;
            match public_archive {
                Ok(bytes) => match PublicArchive::from_bytes(bytes) {
                    Ok(public_archive) => {
                        info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                        Ok(rmp_serde::to_vec(&Archive::build_from_public_archive(public_archive)).expect("Failed to serialize archive"))
                    },
                    Err(err) => Err(foyer::Error::other(format!("Failed to create archive from public archive at [{}] from hybrid cache: {:?}", addr.to_hex(), err))),
                },
                Err(_) => match tarchive {
                    Ok(bytes) => {
                        info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                        Ok(rmp_serde::to_vec(&Archive::build_from_tar(&addr, bytes)).expect("Failed to serialize archive"))
                    },
                    Err(err) => Err(foyer::Error::other(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", addr.to_hex(), err))),
                }
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved archive for [{}] from hybrid cache", addr.to_hex());
                match rmp_serde::from_slice(cache_entry.value()) {
                    Ok(archive) => Ok(archive),
                    Err(e) => {
                        debug!("Failed to deserialize archive for [{}] from hybrid cache: {:?}. Removing entry and retrying.", addr.to_hex(), e);
                        self.hybrid_cache.remove(cache_entry.key());
                        Box::pin(self.archive_get(addr)).await
                    }
                }
            },
            Err(e) => Err(decode::Error::Uncategorized(e.to_string())),
        }
    }

    /// Fetch an archive from the network
    pub async fn archive_get_public(&self, archive_address: ArchiveAddress) -> Result<PublicArchive, decode::Error> {
        match self.data_get_public(&archive_address).await {
            Ok(bytes) => PublicArchive::from_bytes(bytes),
            Err(err) => Err(decode::Error::Uncategorized(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", archive_address.to_hex(), err))),
        }
    }

    pub async fn archive_put_public(&self, archive: &PublicArchive, payment_option: PaymentOption) -> Result<(AttoTokens, ArchiveAddress), PutError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                debug!("creating archive public async");
                client.archive_put_public(archive, payment_option).await
            },
            None => Err(PutError::Serialization(format!("network offline")))
        }
    }

    pub async fn data_get_public(&self, addr: &DataAddress) -> Result<Bytes, GetError> {
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(format!("pd{}", local_address.to_hex()), || async move {
            // todo: optimise range_to to first chunk length (to avoid downloading other chunks when not needed)
            let maybe_bytes = local_caching_client.download_stream(local_address, 0, 524288).await;
            match maybe_bytes {
                Ok(bytes) => {
                    match PublicArchive::from_bytes(bytes.clone()) {
                        // confirm that serialisation can be successful, before returning the data
                        Ok(_) => {
                            info!("retrieved public archive for [{}] from network - storing in hybrid cache", local_address.to_hex());
                            info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                            Ok(Vec::from(bytes))
                        },
                        Err(err) => {
                            error!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), err);
                            Err(foyer::Error::other(format!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), err)))
                        }
                    }
                },
                Err(err) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network {:?}", local_address.to_hex(), err)))
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved public archive for [{}] from hybrid cache", addr.to_hex());
                Ok(Bytes::from(cache_entry.value().to_vec()))
            },
            Err(_) => Err(GetError::RecordNotFound),
        }
    }

    pub async fn get_archive_from_tar(&self, addr: &DataAddress) -> Result<Bytes, GetError> {
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(format!("tar{}", local_address.to_hex()), || async move {
            // todo: confirm whether checking header for tar signature improves performance/reliability
            // 20480
            let trailer_bytes = local_caching_client.download_stream(local_address, -20480, 0).await;
            match trailer_bytes {
                Ok(trailer_bytes) => {
                    match CachingClient::find_subsequence(trailer_bytes.iter().as_slice(), ARCHIVE_TAR_IDX_BYTES) {
                        Some(idx) => {
                            debug!("archive.tar.idx was found in archive.tar");
                            let archive_idx_range_start = idx + 512 + 1;
                            let archive_idx_range_to = 20480;
                            info!("retrieved tarchive for [{}] with range_from [{}] and range_to [{}] from network - storing in hybrid cache", local_address.to_hex(), archive_idx_range_start, archive_idx_range_to);
                            info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                            Ok(Vec::from(&trailer_bytes[archive_idx_range_start..archive_idx_range_to]))
                        },
                        None => {
                            debug!("no archive.tar.idx found in tar trailer");
                            Err(foyer::Error::other(format!("Failed to retrieve archive.tar.idx in tar trailer for [{}] from network", local_address.to_hex())))
                        }
                    }
                },
                Err(err) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network {:?}", local_address.to_hex(), err)))
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved tarchive for [{}] from hybrid cache", addr.to_hex());
                Ok(Bytes::from(cache_entry.value().to_vec()))
            },
            Err(_) => Err(GetError::RecordNotFound),
        }
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

    // todo: is this needed? see above
    /*pub async fn is_tarchive(&self, xor_name: XorName, total_size: usize, data_map: &DataMap) -> bool {
        // https://www.gnu.org/software/tar/manual/html_node/Standard.html
        if total_size > 512 {
            let tar_magic = self.download_stream(xor_name, data_map.clone(), 257, 261).await.to_vec();
            String::from_utf8(tar_magic.clone()).unwrap_or(String::new()) == "ustar"
        } else {
            false
        }
    }*/

    pub async fn pointer_create(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, PointerAddress), PointerError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let owner_clone = owner.clone();
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("creating pointer async");
                    client.pointer_create(&owner_clone, target, payment_option).await
                });
                let address = PointerAddress::new(owner.public_key());
                Ok((AttoTokens::zero(), address))
            },
            None => Err(PointerError::Serialization) // todo: improve error type
        }
    }

    pub async fn pointer_update(
        &self,
        owner: &SecretKey,
        target: PointerTarget,
    ) -> Result<(), PointerError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                let owner_clone = owner.clone();
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("updating pointer async");
                    client.pointer_update(&owner_clone, target).await
                });
                Ok(())
            },
            None => {
                Err(PointerError::Serialization) // todo: improve error type
            }
        }
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

    pub async fn chunk_put(
        &self,
        chunk: &Chunk,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, ChunkAddress), PutError> {
        let chunk_clone = chunk.clone();
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("creating chunk async");
                    client.chunk_put(&chunk_clone, payment_option).await
                });
                Ok((AttoTokens::zero(), chunk.address))
            },
            None => Err(PutError::Serialization(format!("network offline")))
        }
    }

    pub async fn graph_entry_put(
        &self,
        entry: GraphEntry,
        payment_option: PaymentOption,
    ) -> Result<(AttoTokens, GraphEntryAddress), GraphError> {
        let address = entry.address();
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                // todo: move to job processor
                tokio::spawn(async move {
                    debug!("creating graph entry async");
                    client.graph_entry_put(entry, payment_option).await
                });
                Ok((AttoTokens::zero(), address))
            },
            None => Err(GraphError::Serialization(format!("network offline")))
        }
    }

    pub async fn graph_entry_get(
        &self,
        address: &GraphEntryAddress,
    ) -> Result<GraphEntry, GraphError> {
        let local_address = address.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        let local_ant_tp_config = self.ant_tp_config.clone();
        match self.hybrid_cache.get_ref().fetch(format!("gg{}", local_address.to_hex()), {
            let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
            || async move {
                match maybe_local_client {
                    Some(client) => {
                        match client.graph_entry_get(&local_address).await {
                            Ok(scratchpad) => {
                                debug!("found graph entry for address [{}]", local_address.to_hex());
                                info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                                let cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                                Ok(rmp_serde::to_vec(&cache_item).expect("Failed to serialize graph entry"))
                            }
                            Err(_) => Err(foyer::Error::other(format!("Failed to retrieve graph entry for [{}] from network", local_address.to_hex())))
                        }
                    },
                    None => Err(foyer::Error::other(format!("Failed to retrieve graph entry for [{}] from offline network", local_address.to_hex())))
                }
            }
        }).await {
            Ok(cache_entry) => {
                let cache_item: CacheItem<GraphEntry> = rmp_serde::from_slice(cache_entry.value()).expect("Failed to deserialize graph entry");
                info!("retrieved graph entry for [{}] from hybrid cache", address.to_hex());
                if cache_item.has_expired() {
                    // update cache in the background
                    let local_address = address.clone();
                    let local_hybrid_cache = self.hybrid_cache.clone();
                    tokio::spawn({
                        let maybe_local_client = self.client_harness.get_ref().lock().await.get_client().await;
                        async move {
                            match maybe_local_client {
                                Some(client) => {
                                    info!("refreshing hybrid cache with graph entry for [{}] from network, timestamp [{}], ttl [{}]", local_address.to_hex(), cache_item.timestamp, cache_item.ttl);
                                    match client.graph_entry_get(&local_address).await {
                                        Ok(scratchpad) => {
                                            let new_cache_item = CacheItem::new(Some(scratchpad.clone()), local_ant_tp_config.cached_mutable_ttl);
                                            local_hybrid_cache.insert(
                                                format!("gg{}", local_address.to_hex()),
                                                rmp_serde::to_vec(&new_cache_item).expect("Failed to serialize graph entry")
                                            );
                                            info!("inserted hybrid cache with graph entry for [{}] from network", local_address.to_hex());
                                        }
                                        Err(e) => warn!("Failed to refresh expired graph entry for [{}] from network [{}]", local_address.to_hex(), e)
                                    }
                                },
                                None => warn!("Failed to refresh expired graph entry for [{}] from offline network", local_address.to_hex())
                            }
                        }
                    });
                }
                // return last value
                Ok(cache_item.item.unwrap())
            },
            Err(_) => Err(GraphError::Serialization(format!("network offline"))),
        }
    }

    pub async fn download_stream(
        &self,
        addr: DataAddress,
        range_from: i64,
        range_to: i64,
    ) -> Result<Bytes, Error> {
        match self.chunk_get(&ChunkAddress::new(*addr.xorname())).await {
            Ok(data_map_chunk) => {
                let chunk_streamer = ChunkStreamer::new(addr.to_hex(), data_map_chunk.value, self.clone(), self.ant_tp_config.download_threads);
                let total_size = chunk_streamer.get_stream_size().await;
                let derived_range_from: u64 = if range_from < 0 {
                    let size = u64::try_from(total_size).unwrap();
                    let from = u64::try_from(range_from.abs()).unwrap();
                    if from < size {
                        size - from
                    } else {
                        0
                    }
                } else {
                    u64::try_from(range_from).unwrap()
                };
                let derived_range_to: u64 = if range_to <= 0 {
                    let size = u64::try_from(total_size).unwrap();
                    let to= u64::try_from(range_to.abs()).unwrap();
                    if to < size {
                        size - to
                    } else {
                        0
                    }
                } else {
                    u64::try_from(range_to).unwrap()
                };

                let mut chunk_receiver = match chunk_streamer.open(derived_range_from, derived_range_to).await {
                    Ok(chunk_receiver) => chunk_receiver,
                    Err(e) => return Err(ErrorInternalServerError(format!("failed to open chunk stream: {}", e))),
                };

                let mut buf = BytesMut::with_capacity(usize::try_from(derived_range_to - derived_range_from).expect("Failed to convert range from u64 to usize"));
                let mut has_data = true;
                while has_data {
                    match chunk_receiver.next().await {
                        Some(item) => match item {
                            Ok(bytes) => buf.put(bytes),
                            Err(e) => {
                                error!("Error downloading stream from data address [{}] with range [{} - {}]: {}", addr.to_hex(), derived_range_from, derived_range_to, e);
                                has_data = false
                            },
                        },
                        None => has_data = false
                    };
                }
                Ok(buf.freeze())
            }
            Err(e) => Err(ErrorInternalServerError(format!("Failed to download data map chunk: [{}]", e))),
        }
    }

    pub async fn file_content_upload_public(&self, path: PathBuf, payment_option: PaymentOption) -> Result<(AttoTokens, DataAddress), UploadError> {
        match self.client_harness.get_ref().lock().await.get_client().await {
            Some(client) => {
                debug!("file content upload public async");
                client.file_content_upload_public(path, payment_option).await
            },
            None => Err(UploadError::PutError(PutError::Serialization(format!("network offline"))))
        }
    }
}