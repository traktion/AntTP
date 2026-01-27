use std::fmt::Debug;
use actix_http::header::{HeaderMap, IF_NONE_MATCH};
use actix_web::web::Data;
use autonomi::{ChunkAddress, PointerAddress, PublicKey};
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::register::{RegisterAddress};
use log::{debug, error, info};
use xor_name::XorName;
use crate::client::{ArchiveCachingClient, PointerCachingClient, RegisterCachingClient};
use crate::model::archive::Archive;
use crate::service::access_checker::AccessChecker;
use crate::service::pointer_name_resolver::PointerNameResolver;
use crate::service::bookmark_resolver::BookmarkResolver;

pub struct ResolvedAddress {
    pub is_found: bool,
    pub archive: Option<Archive>,
    pub xor_name: XorName,
    pub file_path: String,
    pub is_resolved_from_mutable: bool,
    pub is_modified: bool,
    pub is_allowed: bool,
    pub ttl: u64,
}

impl ResolvedAddress {
    pub fn new(is_found: bool, archive: Option<Archive>, xor_name: XorName, file_path: String, is_resolved_from_mutable: bool, is_modified: bool, is_allowed: bool, ttl: u64) -> Self {
        ResolvedAddress { is_found, archive, xor_name, file_path, is_resolved_from_mutable, is_modified, is_allowed, ttl }
    }
}

#[derive(Debug, Clone)]
pub struct ResolverService {
    archive_caching_client: ArchiveCachingClient,
    pointer_caching_client: PointerCachingClient,
    register_caching_client: RegisterCachingClient,
    access_checker: Data<tokio::sync::Mutex<AccessChecker>>,
    bookmark_resolver: Data<tokio::sync::Mutex<BookmarkResolver>>,
    pointer_name_resolver: Data<PointerNameResolver>,
    ttl_default: u64,
}

#[cfg(test)]
mockall::mock! {
    #[derive(Debug)]
    pub ResolverService {
        pub fn new(archive_caching_client: ArchiveCachingClient,
                   pointer_caching_client: PointerCachingClient,
                   register_caching_client: RegisterCachingClient,
                   access_checker: Data<tokio::sync::Mutex<AccessChecker>>,
                   bookmark_resolver: Data<tokio::sync::Mutex<BookmarkResolver>>,
                   pointer_name_resolver: Data<PointerNameResolver>,
                   ttl_default: u64,
        ) -> Self;
        pub async fn resolve(&self,
                             hostname: &str,
                             path: &str,
                             headers: &HeaderMap
        ) -> Option<ResolvedAddress>;
        pub fn is_immutable_address(&self, chunk_address: &String) -> bool;
        pub fn is_mutable_address(&self, hex_address: &String) -> bool;
        pub async fn resolve_bookmark(&self, name: &String) -> Option<String>;
        pub async fn resolve_name(&self, name: &String) -> Option<String>;
    }
    impl Clone for ResolverService {
        fn clone(&self) -> Self;
    }
}

impl ResolverService {
    pub fn new(archive_caching_client: ArchiveCachingClient,
               pointer_caching_client: PointerCachingClient,
               register_caching_client: RegisterCachingClient,
               access_checker: Data<tokio::sync::Mutex<AccessChecker>>,
               bookmark_resolver: Data<tokio::sync::Mutex<BookmarkResolver>>,
               pointer_name_resolver: Data<PointerNameResolver>,
               ttl_default: u64,
    ) -> ResolverService {
        ResolverService { archive_caching_client, pointer_caching_client, register_caching_client, access_checker, bookmark_resolver, pointer_name_resolver, ttl_default }
    }

    pub async fn resolve(&self,
                         hostname: &str,
                         path: &str,
                         headers: &HeaderMap
    ) -> Option<ResolvedAddress> {
        let path_parts = self.get_path_parts(&hostname, &path).await;
        let (archive_addr, archive_file_name, file_path) = self.assign_path_parts(&path_parts);
        let is_allowed_default = self.access_checker.lock().await.is_allowed_default();
        self.resolve_archive_or_file(
            &archive_addr, &archive_file_name, &file_path, false, is_allowed_default, headers, 0, self.ttl_default).await
    }

    async fn resolve_archive_or_file(
        &self,
        archive_directory: &String,
        archive_file_name: &String,
        archive_file_path: &String,
        is_resolved_from_mutable: bool,
        is_allowed: bool,
        headers: &HeaderMap,
        iteration: usize,
        ttl: u64,
    ) -> Option<ResolvedAddress> {
        if iteration > 10 {
            error!("cyclic reference loop - resolve aborting");
            None
        } else if self.is_bookmark(archive_directory).await {
            debug!("found bookmark for [{}]", archive_directory);
            let resolved_address = &self.resolve_bookmark(archive_directory).await.unwrap_or_default();
            let is_allowed = is_allowed || self.is_allowed(archive_directory).await;
            Box::pin(self.resolve_archive_or_file(
                resolved_address, archive_file_name, archive_file_path, true, is_allowed, headers, iteration + 1, ttl)).await
        } else if self.is_bookmark(archive_file_name).await {
            debug!("found bookmark for [{}]", archive_file_name);
            let resolved_address = &self.resolve_bookmark(archive_file_name).await.unwrap_or_default();
            let is_allowed = is_allowed || self.is_allowed(archive_file_name).await;
            Box::pin(self.resolve_archive_or_file(
                archive_directory, resolved_address, archive_file_path, true, is_allowed, headers, iteration + 1, ttl)).await
        } else if self.is_mutable_address(&archive_directory) {
            debug!("found mutable address for [{}]", archive_directory);
            let is_allowed = is_allowed || self.is_allowed(archive_directory).await;
            match self.analyze_simple(archive_directory).await {
                Some(data_address) => {
                    Box::pin(self.resolve_archive_or_file(
                        &data_address.to_hex(), archive_file_name, archive_file_path, true, is_allowed, headers, iteration + 1, ttl)).await
                }
                None => None
            }
        } else if self.is_immutable_address(&archive_directory) {
            debug!("found immutable address for [{}]", archive_directory);
            let archive_address = match ArchiveAddress::from_hex(archive_directory) {
                Ok(archive_address) => archive_address,
                Err(_) => return None
            };
            let archive_directory_xor_name = archive_address.xorname().clone();
            let is_modified = self.is_modified(headers, &archive_directory);
            let is_allowed = is_allowed || self.is_allowed(archive_directory).await;

            if !is_modified || !is_allowed {
                Some(ResolvedAddress::new(true, None, archive_directory_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed, ttl))
            } else {
                match self.archive_caching_client.archive_get(archive_address).await {
                    Ok(archive) => {
                        debug!("Found archive at [{:x}]", archive_directory_xor_name);
                        Some(ResolvedAddress::new(true, Some(archive), archive_directory_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed, ttl))
                    }
                    Err(_) => {
                        info!("Found XOR address at [{:x}]", archive_directory_xor_name);
                        Some(ResolvedAddress::new(true, None, archive_directory_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed, ttl))
                    }
                }
            }
        } else if self.is_immutable_address(&archive_file_name) {
            let archive_address = match ChunkAddress::from_hex(archive_file_name) {
                Ok(archive_address) => archive_address,
                Err(_) => return None
            };
            let archive_file_name_xor_name = archive_address.xorname().clone();
            let is_modified = self.is_modified(headers, &archive_file_name);
            let is_allowed = is_allowed || self.is_allowed(archive_file_name).await;
            info!("Found XOR address at [{:x}]", archive_file_name_xor_name);
            Some(ResolvedAddress::new(true, None, archive_file_name_xor_name, archive_file_path.clone(), is_resolved_from_mutable, is_modified, is_allowed, ttl))
        } else if let Some(resolved_address) = self.pointer_name_resolver.resolve(archive_directory).await {
            debug!("found PNR record for [{}]", archive_directory);

            let is_allowed = is_allowed || self.is_allowed(archive_file_name).await;
            Box::pin(self.resolve_archive_or_file(
                &resolved_address.address, archive_file_name, archive_file_path, true, is_allowed, headers, iteration + 1, resolved_address.ttl)).await
        } else if let Some(resolved_address) = self.pointer_name_resolver.resolve(archive_file_name).await {
            debug!("found PNR record for [{}]", archive_file_name);

            let is_allowed = is_allowed || self.is_allowed(archive_file_name).await;
            Box::pin(self.resolve_archive_or_file(
                archive_directory, &resolved_address.address, archive_file_path, true, is_allowed, headers, iteration + 1, resolved_address.ttl)).await
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
        match PointerAddress::from_hex(address) {
            Ok(pointer_address) => match self.pointer_caching_client.pointer_get(&pointer_address).await.ok() {
                Some(pointer) => {
                    info!("Analyze found pointer at address [{}] with target [{}]", address, pointer.clone().target().to_hex());
                    Some(DataAddress::from_hex(pointer.clone().target().to_hex().as_str()).unwrap())
                }
                None => {
                    match self.register_caching_client.register_get(&RegisterAddress::from_hex(address).unwrap()).await.ok() {
                        Some(register_value) => {
                            info!("Analyze found register at address [{}] with value [{}]", address, hex::encode(register_value.clone()));
                            Some(DataAddress::from_hex(hex::encode(register_value.clone()).as_str()).unwrap())
                        }
                        None => None
                    }
                }
            }
            Err(_) => None
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
    
    pub fn is_immutable_address(&self, chunk_address: &String) -> bool {
        chunk_address.len() == 64 && ChunkAddress::from_hex(chunk_address).ok().is_some()
    }

    pub fn is_mutable_address(&self, hex_address: &String) -> bool {
        hex_address.len() == 96 && PublicKey::from_hex(hex_address).ok().is_some()
    }

    async fn is_bookmark(&self, name: &String) -> bool {
        self.bookmark_resolver.lock().await.is_bookmark(name)
    }

    pub async fn resolve_bookmark(&self, name: &String) -> Option<String> {
        self.bookmark_resolver.lock().await.resolve(name)
    }

    pub async fn resolve_name(&self, name: &String) -> Option<String> {
        match self.resolve_bookmark(name).await {
            Some(resolved_address) => Some(resolved_address.to_string()),
            None => match self.pointer_name_resolver.resolve(name).await {
                Some(resolved_address) => Some(resolved_address.address.to_string()),
                None => None
            }
        }
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

    async fn get_path_parts(&self, hostname: &str, path: &str) -> Vec<String> {
        // assert: <address>.any.domain.name as acceptable format
        let hostname_parts = hostname.split(".").map(|s| s.to_string()).collect::<Vec<String>>();
        let address = if hostname_parts.len() > 1 {
            hostname_parts[0].clone()
        } else {
            hostname.to_string()
        };
        if self.is_valid_address(&address).await {
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

    async fn is_valid_address(&self, address: &String) -> bool {
        // todo: convert to proxy enabled check?
        self.is_immutable_address(address)
            || self.is_mutable_address(address)
            || self.is_bookmark(address).await
            || self.pointer_name_resolver.is_resolved(address).await
    }
}
