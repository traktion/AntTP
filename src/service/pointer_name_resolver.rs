use ant_protocol::storage::{ChunkAddress, Pointer, PointerAddress, PointerTarget};
use autonomi::{Client, SecretKey};
use log::{debug, error, warn};
use crate::client::{ChunkCachingClient, PointerCachingClient};
use crate::error::GetError;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::PnrZone;

pub struct ResolvedRecord {
    pub address: String,
    pub ttl: u64,
}

impl ResolvedRecord {
    pub fn new(address: String, ttl: u64) -> Self {
        Self { address, ttl }
    }
}

#[derive(Debug)]
pub struct PointerNameResolver {
    pointer_caching_client: PointerCachingClient,
    chunk_caching_client: ChunkCachingClient,
    pointer_name_resolver_secret_key: SecretKey,
    ttl_default: u64,
}

impl PointerNameResolver {
    pub fn new(pointer_caching_client: PointerCachingClient, chunk_caching_client: ChunkCachingClient, pointer_name_resolver_secret_key: SecretKey, ttl_default: u64) -> PointerNameResolver {
        PointerNameResolver { pointer_caching_client, chunk_caching_client, pointer_name_resolver_secret_key, ttl_default }
    }

    pub async fn is_resolved(&self, name: &String) -> bool {
        self.resolve(name).await.is_some()
    }

    pub async fn resolve(&self, name: &String) -> Option<ResolvedRecord> {
        if name.is_empty() {
            None
        } else {
            debug!("get key from name: {}", name);
            let pointer_key = Client::register_key_from_name(&self.pointer_name_resolver_secret_key, name.as_str());
            debug!("found: name={}, pointer_key={}, public_key={}", name, pointer_key.to_hex(), &pointer_key.public_key().to_hex());
            match self.resolve_pointer(&pointer_key.public_key().to_hex(), 0).await.ok() {
                Some(pointer) => match self.resolve_map(&pointer.target().to_hex()).await {
                    Some(resolved_record) => {
                        // use resolved record TTL to update TTLs for cache
                        self.update_pointer_ttls(&pointer_key.public_key().to_hex(), resolved_record.ttl, 0).await.ok()?;
                        Some(resolved_record) // return map target
                    },
                    None => Some(ResolvedRecord::new(pointer.target().to_hex(), self.ttl_default )) // return pointer target
                }
                None => None,
            }
        }
    }

    async fn resolve_pointer(&self, address: &String, iteration: usize) -> Result<Pointer, PointerError> {
        debug!("resolve_pointer: address={}, iteration={}", address, iteration);
        if iteration > 10 {
            error!("cyclic reference loop - resolve aborting");
            Err(PointerError::GetError(GetError::RecordNotFound(format!("Too many iterations which resolving: {}", address))))
        } else {
            match PointerAddress::from_hex(address) {
                Ok(pointer_address) => match self.pointer_caching_client.pointer_get(&pointer_address).await {
                    Ok(pointer) => match pointer.target() {
                        PointerTarget::ChunkAddress(_) => Ok(pointer),
                        _ => Box::pin(self.resolve_pointer(&pointer.target().to_hex(), iteration + 1)).await,
                    }
                    Err(_) => Err(PointerError::GetError(GetError::RecordNotFound(format!("Not found: {}", address))))
                }
                Err(_) => Err(PointerError::GetError(GetError::RecordNotFound(format!("Not found: {}", address))))
            }
        }
    }

    async fn update_pointer_ttls(&self, address: &String, ttl_override: u64, iteration: usize) -> Result<Pointer, PointerError> {
        debug!("update_pointer_ttls: address={}, iteration={}, ttl_override={}", address, iteration, ttl_override);
        if iteration > 10 {
            error!("cyclic reference loop - resolve aborting");
            Err(PointerError::GetError(GetError::RecordNotFound(format!("Too many iterations which resolving: {}", address))))
        } else {
            match PointerAddress::from_hex(address) {
                Ok(pointer_address) => {
                    match self.pointer_caching_client.pointer_get(&pointer_address).await {
                        Ok(pointer) => {
                            self.pointer_caching_client.pointer_update_ttl(&pointer.address(), ttl_override).await?;
                            match pointer.target() {
                                PointerTarget::ChunkAddress(_) => Ok(pointer),
                                _ => Box::pin(self.update_pointer_ttls(&pointer.target().to_hex(), ttl_override, iteration + 1)).await,
                            }
                        }
                        Err(_) => Err(PointerError::GetError(GetError::RecordNotFound(format!("Not found: {}", address))))
                    }
                }
                Err(_) => Err(PointerError::GetError(GetError::RecordNotFound(format!("Not found: {}", address))))
            }
        }
    }

    pub async fn resolve_map(&self, name: &String) -> Option<ResolvedRecord> {
        match ChunkAddress::from_hex(&name) {
            Ok(chunk_address) => match self.chunk_caching_client.chunk_get_internal(&chunk_address).await {
                Ok(chunk) => {
                    match serde_json::from_slice::<PnrZone>(&chunk.value) {
                        Ok(pnr_zone) => {
                            debug!("deserialized {} PNR records", pnr_zone.records.len());
                            for pnr_record in pnr_zone.records {
                                if pnr_record.sub_name.unwrap_or("".to_string()).is_empty() {
                                    debug!("found default PNR record");
                                    return Some(ResolvedRecord::new(pnr_record.address.to_string(), pnr_record.ttl));
                                }
                            }
                            debug!("no default PNR record found");
                            None
                        }
                        Err(e) => {
                            warn!("failed to deserialize chunk as PNR records: {}", e);
                            None
                        }
                    }
                },
                Err(e) => {
                    warn!("failed to get chunk content: {:?}", e);
                    None
                }
            },
            Err(e) => {
                warn!("failed to get chunk_address from name: {:?}", e);
                None
            }
        }
    }
}