use actix_http::header::{HeaderMap, IF_NONE_MATCH};
use actix_web::web::Data;
use autonomi::{ChunkAddress, PointerAddress, PublicKey};
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::register::{RegisterAddress};
use log::{debug, info};
use tokio::sync::Mutex;
use xor_name::XorName;
use crate::config::anttp_config::AntTpConfig;
use crate::client::CachingClient;
use crate::model::archive::Archive;
use crate::service::access_checker::AccessChecker;

pub struct ResolvedAddress {
    pub is_found: bool,
    pub archive: Option<Archive>,
    pub xor_name: XorName,
    pub file_path: String,
    pub is_resolved_from_mutable: bool,
    pub is_modified: bool,
    pub is_allowed: bool,
}

impl ResolvedAddress {
    pub fn new(is_found: bool, archive: Option<Archive>, xor_name: XorName, file_path: String, is_resolved_from_mutable: bool, is_modified: bool, is_allowed: bool) -> Self {
        ResolvedAddress { is_found, archive, xor_name, file_path, is_resolved_from_mutable, is_modified, is_allowed }
    }
}

pub struct ResolverService {
    ant_tp_config: AntTpConfig,
    caching_client: CachingClient,
    access_checker: Data<Mutex<AccessChecker>>,
}

impl ResolverService {
    pub fn new(ant_tp_config: AntTpConfig, caching_client: CachingClient, access_checker: Data<Mutex<AccessChecker>>) -> ResolverService {
        ResolverService { ant_tp_config, caching_client, access_checker }
    }

    pub async fn resolve(&self,
                         hostname: &str,
                         path: &str,
                         headers: &HeaderMap
    ) -> Option<ResolvedAddress> {
        let path_parts = self.get_path_parts(&hostname, &path);
        let (archive_addr, archive_file_name, file_path) = self.assign_path_parts(&path_parts);
        self.resolve_archive_or_file(
            &archive_addr, &archive_file_name, &file_path, false, headers).await
    }

    async fn resolve_archive_or_file(
        &self,
        archive_directory: &String,
        archive_file_name: &String,
        archive_file_path: &String,
        is_resolved_from_mutable: bool,
        headers: &HeaderMap
    ) -> Option<ResolvedAddress> {
        if self.is_bookmark(archive_directory) {
            debug!("found bookmark for [{}]", archive_directory);
            let resolved_bookmark = &self.resolve_bookmark(archive_directory).unwrap().to_string();
            Box::pin(self.resolve_archive_or_file(
                resolved_bookmark, archive_file_name, archive_file_path, true, headers)).await
        } else if self.is_bookmark(archive_file_name) {
            debug!("found bookmark for [{}]", archive_file_name);
            let resolved_bookmark = &self.resolve_bookmark(archive_file_name).unwrap().to_string();
            Box::pin(self.resolve_archive_or_file(
                archive_directory, resolved_bookmark, archive_file_path, true, headers)).await
        } else if self.is_mutable_address(&archive_directory) {
            debug!("found mutable address for [{}]", archive_directory);
            match self.analyze_simple(archive_directory).await {
                Some(data_address) => {
                    Box::pin(self.resolve_archive_or_file(
                        &data_address.to_hex(), archive_file_name, archive_file_path, true, headers)).await
                }
                None => None
            }
        } else if self.is_immutable_address(&archive_directory) {
            debug!("found immutable address for [{}]", archive_directory);
            let archive_address = ArchiveAddress::from_hex(archive_directory).unwrap();
            let archive_directory_xor_name = archive_address.xorname().clone();
            let is_modified = self.is_modified(headers, &archive_directory);
            let is_allowed = self.is_allowed(archive_directory).await;


            if !is_modified || !is_allowed {
                Some(ResolvedAddress::new(true, None, archive_directory_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed))
            } else {
                match self.caching_client.archive_get(archive_address).await {
                    Ok(archive) => {
                        debug!("Found archive at [{:x}]", archive_directory_xor_name);
                        Some(ResolvedAddress::new(true, Some(archive), archive_directory_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed))
                    }
                    Err(_) => {
                        info!("Found XOR address at [{:x}]", archive_directory_xor_name);
                        Some(ResolvedAddress::new(true, None, archive_directory_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed))
                    }
                }
            }
        } else if self.is_immutable_address(&archive_file_name) {
            let archive_file_name_xor_name = ChunkAddress::from_hex(archive_file_name).unwrap().xorname().clone();
            let is_modified = self.is_modified(headers, &archive_file_name);
            let is_allowed = self.is_allowed(archive_file_name).await;
            info!("Found XOR address at [{:x}]", archive_file_name_xor_name);
            Some(ResolvedAddress::new(true, None, archive_file_name_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed))
        } else {
            debug!("Failed to find archive or filename [{:?}]", archive_file_name);
            None
        }
    }

    async fn is_allowed(&self, address: &String) -> bool {
        let access_checker = self.access_checker.lock().await;
        access_checker.is_allowed(address)
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
                    None => None
                }
            }
        }
    }

    fn is_modified(&self, headers: &HeaderMap, target_e_tag: &String) -> bool {
        // todo: should this check content-type too? seeing some json returned on web browser indexes for IMIM data
        if headers.contains_key(IF_NONE_MATCH) && let Some(header_value) = headers.get(IF_NONE_MATCH) {
            let source_e_tag = header_value.to_str().unwrap_or("").trim_matches('"');
            source_e_tag != target_e_tag.as_str()
        } else {
            true
        }
    }
    
    fn is_immutable_address(&self, chunk_address: &String) -> bool {
        chunk_address.len() == 64 && ChunkAddress::from_hex(chunk_address).ok().is_some()
    }

    fn is_mutable_address(&self, hex_address: &String) -> bool {
        hex_address.len() == 96 && PublicKey::from_hex(hex_address).ok().is_some()
    }

    fn is_bookmark(&self, alias: &String) -> bool {
        self.ant_tp_config.bookmarks_map.contains_key(alias)
    }

    pub fn resolve_bookmark(&self, alias: &String) -> Option<String> {
        self.ant_tp_config.bookmarks_map.get(alias).cloned()
    }

    fn assign_path_parts(&self, path_parts: &Vec<String>) -> (String, String, String) {
        if path_parts.len() > 1 {
            (path_parts[0].to_string(), path_parts[1].to_string(), path_parts[1..].join("/").to_string())
        } else if path_parts.len() > 0 {
            (path_parts[0].to_string(), "".to_string(), "".to_string())
        } else {
            ("".to_string(), "".to_string(), "".to_string())
        }
    }

    fn get_path_parts(&self, hostname: &str, path: &str) -> Vec<String> {
        // assert: <address>.any.domain.name as acceptable format
        let hostname_parts = hostname.split(".").map(|s| s.to_string()).collect::<Vec<String>>();
        let address = if hostname_parts.len() > 1 {
            hostname_parts[0].clone()
        } else {
            hostname.to_string()
        };
        if self.is_valid_address(&address) {
            let mut subdomain_parts = Vec::new();
            subdomain_parts.push(address);
            let path_parts = path.split("/")
                .map(str::to_string)
                .collect::<Vec<String>>();
            subdomain_parts.append(&mut path_parts.clone());
            subdomain_parts
        } else {
            path.split("/")
                .map(str::to_string)
                .collect::<Vec<String>>()
        }
    }

    fn is_valid_address(&self, address: &String) -> bool {
        self.is_immutable_address(address) || self.is_mutable_address(address) || self.is_bookmark(address)
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
}