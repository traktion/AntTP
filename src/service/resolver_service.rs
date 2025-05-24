use std::convert::TryInto;
use actix_http::header::{HeaderMap, IF_NONE_MATCH};
use actix_web::Error;
use actix_web::error::{ErrorBadRequest};
use autonomi::{Client, PointerAddress, PublicKey};
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::PublicArchive;
use autonomi::register::RegisterAddress;
use log::{debug, info, warn};
use xor_name::XorName;
use crate::anttp_config::AntTpConfig;
use crate::caching_client::CachingClient;
use crate::archive_helper::{DataState};

pub struct ResolvedAddress {
    pub is_found: bool,
    pub archive: PublicArchive,
    pub is_archive: bool,
    pub xor_addr: XorName,
}

impl ResolvedAddress {
    pub fn new(is_found: bool, archive: PublicArchive, is_archive: bool, xor_addr: XorName) -> Self {
        ResolvedAddress { is_found, archive, is_archive, xor_addr }
    }
}

#[derive(Clone)]
pub struct ResolverService {
    ant_tp_config: AntTpConfig
}

impl ResolverService {
    pub fn new(ant_tp_config: AntTpConfig) -> ResolverService {
        ResolverService { ant_tp_config }
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

    pub async fn resolve_archive_or_file(&self, autonomi_client: Client, caching_autonomi_client: &CachingClient, archive_directory: &String, archive_file_name: &String) -> ResolvedAddress {
        if self.is_bookmark(archive_directory) {
            let resolved_bookmark = &self.resolve_bookmark(archive_directory).unwrap().to_string();
            Box::pin(self.resolve_archive_or_file(autonomi_client, caching_autonomi_client, resolved_bookmark, archive_file_name)).await
        } else if self.is_bookmark(archive_file_name) {
            let resolved_bookmark = &self.resolve_bookmark(archive_file_name).unwrap().to_string();
            Box::pin(self.resolve_archive_or_file(autonomi_client, caching_autonomi_client, archive_directory, resolved_bookmark)).await
        } else if self.is_mutable_address(&archive_directory) {
            info!("Analyze archive_directory [{:?}]", archive_directory);
            // todo: cache result for short period through HTTP caching (return in type?)
            match self.analyze_simple(autonomi_client.clone(), archive_directory).await {
                Some(data_address) => {
                    Box::pin(self.resolve_archive_or_file(autonomi_client, caching_autonomi_client, &data_address.to_hex(), archive_file_name)).await
                }
                None => {
                    let archive_directory_xorname = self.str_to_xor_name(&archive_directory).unwrap();
                    info!("No public archive found at [{:x}]. Treating as XOR address", archive_directory_xorname);
                    ResolvedAddress::new(false, PublicArchive::new(), false, archive_directory_xorname)
                }
            }
        } else if self.is_immutable_address(&archive_directory) {
            let archive_directory_xorname = self.str_to_xor_name(&archive_directory).unwrap();
            let archive_address = ArchiveAddress::new(archive_directory_xorname);
            match caching_autonomi_client.archive_get_public(archive_address).await {
                Ok(public_archive) => {
                    info!("Found public archive at [{:x}]", archive_directory_xorname);
                    ResolvedAddress::new(true, public_archive, true, archive_directory_xorname)
                }
                Err(_) => {
                    info!("No public archive found at [{:x}]. Treating as XOR address", archive_directory_xorname);
                    ResolvedAddress::new(true, PublicArchive::new(), false, archive_directory_xorname)
                }
            }
        }
        else if self.is_immutable_address(&archive_file_name) {
            let archive_file_name_xorname = self.str_to_xor_name(&archive_file_name).unwrap();
            info!("Found XOR address [{:x}]", archive_file_name_xorname);
            ResolvedAddress::new(true, PublicArchive::new(), false, archive_file_name_xorname)
        } else {
            warn!("Failed to find archive or filename [{:?}]", archive_file_name);
            ResolvedAddress::new(false, PublicArchive::new(), false, XorName::default())
        }
    }

    async fn analyze_simple(&self, autonomi_client: Client, address: &String) -> Option<DataAddress> {
        // todo: analyze other types in a performant way - assume only pointers/registers for now
        match autonomi_client.pointer_get(&PointerAddress::from_hex(address).unwrap()).await.ok() {
            Some(pointer) => {
                info!("Analyze found pointer");
                Some(DataAddress::from_hex(pointer.target().to_hex().as_str()).unwrap())
            }
            None => {
                match autonomi_client.register_get(&RegisterAddress::from_hex(address).unwrap()).await.ok() {
                    Some(register_value) => {
                        info!("Analyze found register");
                        Some(DataAddress::from_hex(hex::encode(register_value).as_str()).unwrap())
                    }
                    None => {
                        None
                    }
                }
            }
        }
    }

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
        //if alias == "" || self.is_mutable_address(alias) || self.is_immutable_address(alias) { return false; }
        if alias == "" { return false; }
        info!("Searching for bookmark [{}]", alias.clone());
        let alias_with_delimiter = format!("{}=", alias);
        self.ant_tp_config.bookmarks.iter().filter(|&s| s.starts_with(alias_with_delimiter.as_str())).next().is_some()
        //self.ant_tp_config.bookmarks.contains(&alias)
    }
    
    pub fn resolve_bookmark(&self, alias: &String) -> Option<String> {
        let bookmark = self.ant_tp_config.bookmarks.iter().filter(|&s| s.starts_with(alias.as_str())).next();
        match bookmark {
            Some(bookmark) => {
                let values = bookmark.split("=").collect::<Vec<&str>>();
                match values.get(1) {
                    Some(target) => {
                        info!("Found bookmark [{}] with target [{}]", alias, target.to_string());
                        Some(target.to_string())
                    },
                    None => None
                }
            }
            None => {
                None
            }
        }
    }

    fn str_to_xor_name(&self, str: &String) -> Result<XorName, Error> {
        match hex::decode(str) {
            Ok(bytes) => {
                let xor_name_bytes: [u8; 32] = bytes
                    .try_into()
                    .expect("Failed to parse XorName from hex string");
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