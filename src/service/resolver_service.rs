use std::convert::TryInto;
use actix_http::header::{HeaderMap, IF_NONE_MATCH};
use actix_web::Error;
use actix_web::error::{ErrorBadGateway, ErrorBadRequest};
use autonomi::{PointerAddress, PublicKey};
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::register::{RegisterAddress};
use log::{debug, info, warn};
use xor_name::XorName;
use crate::config::anttp_config::AntTpConfig;
use crate::client::CachingClient;
use crate::model::archive::Archive;
use crate::service::archive_helper::{DataState};

pub struct ResolvedAddress {
    pub is_found: bool,
    pub archive: Option<Archive>,
    pub xor_name: XorName,
    pub is_mutable: bool,
}

impl ResolvedAddress {
    pub fn new(is_found: bool, archive: Option<Archive>, xor_name: XorName, is_mutable: bool) -> Self {
        ResolvedAddress { is_found, archive, xor_name, is_mutable }
    }
}

#[derive(Clone)]
pub struct ResolverService {
    ant_tp_config: AntTpConfig,
    caching_client: CachingClient
}

impl ResolverService {
    pub fn new(ant_tp_config: AntTpConfig, caching_client: CachingClient) -> ResolverService {
        ResolverService { ant_tp_config, caching_client }
    }

    pub fn get_data_state(&self, headers: &HeaderMap, xor_name: &XorName) -> DataState {
        if headers.contains_key(IF_NONE_MATCH) {
            let e_tag = headers.get(IF_NONE_MATCH).unwrap().to_str().unwrap();
            let source_e_tag = e_tag.to_string().replace("\"", "");
            let target_e_tag = format!("{:x}", xor_name);
            debug!("is_modified == [{}], source_e_tag = [{}], target_e_tag = [{}], IF_NONE_MATCH present", source_e_tag == target_e_tag, source_e_tag, target_e_tag);
            if source_e_tag != target_e_tag {
                DataState::Modified
            } else {
                DataState::NotModified
            }
        } else {
            debug!("is_modified == [true], IF_NONE_MATCH absent");
            DataState::Modified
        }
    }

    pub async fn resolve_archive_or_file(&self, archive_directory: &String, archive_file_name: &String, is_resolved_mutable: bool) -> Option<ResolvedAddress> {
        if self.is_bookmark(archive_directory) {
            debug!("found bookmark for [{}]", archive_directory);
            let resolved_bookmark = &self.resolve_bookmark(archive_directory).unwrap().to_string();
            Box::pin(self.resolve_archive_or_file(resolved_bookmark, archive_file_name, true)).await
        } else if self.is_bookmark(archive_file_name) {
            debug!("found bookmark for [{}]", archive_file_name);
            let resolved_bookmark = &self.resolve_bookmark(archive_file_name).unwrap().to_string();
            Box::pin(self.resolve_archive_or_file(archive_directory, resolved_bookmark, true)).await
        } else if self.is_mutable_address(&archive_directory) {
            debug!("found mutable address for [{}]", archive_directory);
            match self.analyze_simple(archive_directory).await {
                Some(data_address) => {
                    Box::pin(self.resolve_archive_or_file(&data_address.to_hex(), archive_file_name, true)).await
                }
                None => {
                    info!("No mutable data found at [{}]", archive_directory);
                    None
                }
            }
        } else if self.is_immutable_address(&archive_directory) {
            debug!("found immutable address for [{}]", archive_directory);
            let archive_directory_xorname = self.str_to_xor_name(&archive_directory).unwrap();

            let archive_address = ArchiveAddress::new(archive_directory_xorname);
            match self.caching_client.archive_get(archive_address).await {
                Ok(archive) => {
                    info!("Found archive at [{:x}]", archive_directory_xorname);
                    Some(ResolvedAddress::new(true, Some(archive), archive_directory_xorname, is_resolved_mutable))
                }
                Err(e) => {
                    info!("No archive found at [{:x}] due to error [{}]. Treating as XOR address", archive_directory_xorname, e.to_string());
                    Some(ResolvedAddress::new(true, None, archive_directory_xorname, is_resolved_mutable))
                }
            }
        } else if self.is_immutable_address(&archive_file_name) {
            let archive_file_name_xorname = self.str_to_xor_name(&archive_file_name).unwrap();
            info!("Found XOR address [{:x}]", archive_file_name_xorname);
            Some(ResolvedAddress::new(true, None, archive_file_name_xorname, is_resolved_mutable))
        } else {
            warn!("Failed to find archive or filename [{:?}]", archive_file_name);
            None
        }
    }

    async fn analyze_simple(&self, address: &String) -> Option<DataAddress> {
        // todo: analyze other types in a performant way - assume only pointers/registers for now
        // todo: could do both + join, but it may slow get pointer response
        match self.caching_client.pointer_get(&PointerAddress::from_hex(address).unwrap()).await.ok() {
            Some(pointer) => {
                info!("Analyze found pointer at address [{}] with target [{}]", address, pointer.clone().target().to_hex());
                Some(DataAddress::from_hex(pointer.clone().target().to_hex().as_str()).unwrap())
            }
            None => {
                match self.caching_client.register_get(&RegisterAddress::from_hex(address).unwrap()).await.ok() {
                    Some(register_value) => {
                        info!("Analyze found register at address [{}] with value [{}]", address, hex::encode(register_value.clone()));
                        Some(DataAddress::from_hex(hex::encode(register_value.clone()).as_str()).unwrap())
                    }
                    None => {
                        None
                    }
                }
            }
        }
    }

    // todo: improve and test to see if reliable performance gains can be achieved
    /*async fn analyze_simple(&self, address: &String) -> Option<DataAddress> {
        let pointer_address = match &PointerAddress::from_hex(address) {
            Ok(address) => address.clone(),
            Err(_) => {
                warn!("Failed to parse pointer address [{}]", address);
                return None
            },
        };
        let register_address = match &RegisterAddress::from_hex(address) {
            Ok(address) => address.clone(),
            Err(_) => {
                warn!("Failed to parse register address [{}]", address);
                return None
            },
        };
        let register_head_pointer = register_address.to_underlying_head_pointer().clone();

        let (is_pointer, is_register) = join!(
            self.caching_client.pointer_check_existence(&pointer_address),
            self.caching_client.pointer_check_existence(&register_head_pointer),
        );

        if is_pointer.unwrap_or(false) {
            match self.caching_client.pointer_get(&pointer_address).await {
                Ok(pointer) => {
                    info!("Analyze found pointer at address [{}] with target [{}]", address, pointer.clone().target().to_hex());
                    match DataAddress::from_hex(pointer.target().to_hex().as_str()) {
                        Ok(address) => Some(address),
                        Err(_) => None,
                    }
                },
                Err(_) => None,
            }
        } else if is_register.unwrap_or(false) {
            match self.caching_client.register_get(&register_address).await {
                Ok(register_value) => {
                    info!("Analyze found register at address [{}] with value [{}]", address, hex::encode(register_value.clone()));
                    match DataAddress::from_hex(hex::encode(register_value).as_str()) {
                        Ok(address) => Some(address),
                        Err(_) => None,
                    }
                },
                Err(_) => None,
            }
        } else {
            None
        }
    }*/

    /*async fn analyze_complex(&self, autonomi_client: Client, address: &String) -> Result<DataAddress, Error> {
        // note: this is an exhaust test and is rather slow
        match autonomi_client.analyze_address(&address, true).await {
            Ok(Analysis::Register { current_value, .. }) => {
                info!("Analyze found register with current value [{}]", &hex::encode(current_value));
                Ok(ArchiveAddress::from_hex(&hex::encode(current_value)).unwrap())
            }
            Ok(Analysis::Pointer(pointer)) => {
                info!("Analyze found pointer");
                Ok(ArchiveAddress::from_hex(pointer.target().to_hex().as_str()).unwrap())
            }
            Ok(Analysis::PublicArchive { address, .. }) => {
                info!("Analyze found public archive");
                Ok(ArchiveAddress::from_hex(address.unwrap().to_hex().as_str()).unwrap())
            }
            Ok(Analysis::Chunk(chunk, ..)) => {
                info!("Analyze found chunk");
                Ok(ArchiveAddress::from_hex(chunk.address.to_hex().as_str()).unwrap())
            }
            //Ok(Analysis::GraphEntry(_)) => {}
            //Ok(Analysis::Scratchpad(_)) => {}
            Ok(Analysis::DataMap { address, .. }) => {
                Ok(ArchiveAddress::from_hex(address.to_hex().as_str()).unwrap())
            }
            //Ok(Analysis::RawDataMap { .. }) => {}
            //Ok(Analysis::PrivateArchive(_)) => {}
            Ok(_) => {
                Err(ErrorNotFound(format!("Unsupported data type [{}]", address)))
            }
            Err(err) => {
                Err(ErrorNotFound(format!("Unknown data type [{}] with error [{}]", address, err)))
            }
        }
    }*/
    
    pub fn is_valid_hostname(&self, hostname: &str) -> bool {
        self.is_immutable_address(&hostname.to_string()) || self.is_mutable_address(&hostname.to_string()) || self.is_bookmark(&hostname.to_string())
    }

    pub fn is_immutable_address(&self, chunk_address: &String) -> bool {
        chunk_address.len() == 64 && autonomi::ChunkAddress::from_hex(chunk_address).ok().is_some()
    }

    pub fn is_mutable_address(&self, hex_address: &String) -> bool {
        hex_address.len() == 96 && PublicKey::from_hex(hex_address).ok().is_some()
    }

    pub fn is_bookmark(&self, alias: &String) -> bool {
        self.ant_tp_config.bookmarks_map.contains_key(alias)
    }
    
    pub fn resolve_bookmark(&self, alias: &String) -> Option<String> {
        self.ant_tp_config.bookmarks_map.get(alias).cloned()
    }

    pub async fn resolve_address(&self, name: &String) -> Option<String> {
        if self.is_bookmark(name) {
            debug!("found bookmark for [{}]", name);
            let resolved_bookmark = &self.resolve_bookmark(name).unwrap().to_string();
            Box::pin(self.resolve_address(resolved_bookmark)).await
        } else if self.is_mutable_address(&name) {
            debug!("found mutable address for [{}]", name);
            match self.analyze_simple(name).await {
                Some(data_address) => {
                    debug!("found immutable address for [{}]", name);
                    Some(data_address.to_hex())
                }
                None => {
                    info!("No mutable data found at [{}]", name);
                    None
                }
            }
        } else if self.is_immutable_address(&name) {
            debug!("found immutable address for [{}]", name);
            Some(name.clone())
        } else {
            warn!("Failed to find name [{:?}]", name);
            None
        }
    }

    fn str_to_xor_name(&self, str: &String) -> Result<XorName, Error> {
        match hex::decode(str) {
            Ok(bytes) => {
                let xor_name_bytes: [u8; 32] = match bytes.try_into() {
                    Ok(bytes) => bytes,
                    Err(_) => return Err(ErrorBadGateway("Failed to parse XorName from hex string")),
                };
                Ok(XorName(xor_name_bytes))
            },
            Err(_) => {
                Err(ErrorBadRequest(format!("Invalid XorName [{}]", str)))
            }
        }
    }

    pub fn assign_path_parts(&self, path_parts: Vec<String>) -> (String, String) {
        if path_parts.len() > 1 {
            (path_parts[0].to_string(), path_parts[1].to_string())
        } else if path_parts.len() > 0 {
            (path_parts[0].to_string(), "".to_string())
        } else {
            ("".to_string(), "".to_string())
        }
    }
}