use ant_protocol::storage::{ChunkAddress, Pointer, PointerAddress, PointerTarget};
use autonomi::{Client, SecretKey};
use log::{debug, error, warn};
#[double]
use crate::client::ChunkCachingClient;
#[double]
use crate::client::PointerCachingClient;
use crate::error::GetError;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::PnrZone;
use mockall_double::double;

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
                            if let Some(pnr_record) = pnr_zone.records.get("") {
                                debug!("found default PNR record");
                                return Some(ResolvedRecord::new(pnr_record.address.to_string(), pnr_record.ttl));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::client::MockChunkCachingClient;
    use crate::client::MockPointerCachingClient;
    use crate::model::pnr::{PnrRecord, PnrRecordType};
    use crate::error::chunk_error::ChunkError;
    use autonomi::{Chunk, ChunkAddress};
    use bytes::Bytes;
    use mockall::predicate::*;

    fn create_test_resolver(
        mock_pointer_caching_client: MockPointerCachingClient,
        mock_chunk_caching_client: MockChunkCachingClient,
    ) -> PointerNameResolver {
        let secret_key = SecretKey::random();
        PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, secret_key, 3600)
    }

    #[tokio::test]
    async fn test_resolve_empty_name() {
        let mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);

        assert!(resolver.resolve(&"".to_string()).await.is_none());
    }

    #[tokio::test]
    async fn test_resolve_pointer_not_found() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(|_| Err(PointerError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve(&"test.name".to_string()).await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_resolve_pointer_to_chunk_directly() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let target_chunk_address = ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap();
        let pointer = Pointer::new(&SecretKey::random(), 0, PointerTarget::ChunkAddress(target_chunk_address));
        
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |_| Ok(pointer.clone()));

        // resolve_map fails (no chunk found)
        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .returning(|_| Err(ChunkError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve(&"test.name".to_string()).await;

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.address, target_chunk_address.to_hex());
        assert_eq!(record.ttl, 3600);
    }

    #[tokio::test]
    async fn test_resolve_pointer_to_pnr_map() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let map_chunk_address = ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap();
        let pointer = Pointer::new(&SecretKey::random(), 0, PointerTarget::ChunkAddress(map_chunk_address));
        
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |_| Ok(pointer.clone()));

        mock_pointer_caching_client
            .expect_pointer_update_ttl()
            .returning(|_, _| Ok(Pointer::new(&SecretKey::random(), 0, PointerTarget::ChunkAddress(ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap()))));

        let resolved_address = "b40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527".to_string();
        let pnr_zone = PnrZone::new(
            "test.name".to_string(),
            HashMap::from([("".to_string(), PnrRecord::new(resolved_address.clone(), PnrRecordType::A, 60))]),
            None,
            None
        );
        let chunk_value = serde_json::to_vec(&pnr_zone).unwrap();
        let chunk = Chunk::new(Bytes::from(chunk_value));

        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .returning(move |_| Ok(chunk.clone()));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve(&"test.name".to_string()).await;

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.address, resolved_address);
        assert_eq!(record.ttl, 60);
    }

    #[tokio::test]
    async fn test_resolve_pointer_chain() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let sk1 = SecretKey::random();
        let sk2 = SecretKey::random();
        let addr2 = PointerAddress::from_hex(&sk2.public_key().to_hex()).unwrap();
        let target_chunk_address = ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap();
        
        let p1 = Pointer::new(&sk1, 0, PointerTarget::PointerAddress(addr2));
        let p2 = Pointer::new(&sk2, 0, PointerTarget::ChunkAddress(target_chunk_address));

        let p1_clone = p1.clone();
        let p2_clone = p2.clone();

        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |addr: &PointerAddress| {
                if addr.to_hex() == p1_clone.address().to_hex() {
                    Ok(p1_clone.clone())
                } else if addr.to_hex() == p2_clone.address().to_hex() {
                    Ok(p2_clone.clone())
                } else {
                    Err(PointerError::GetError(GetError::RecordNotFound("Not found".to_string())))
                }
            });

        let resolver = PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, sk1, 3600);
        // We need to know the name that hashes to sk1. 
        // But Client::register_key_from_name is deterministic.
        // Let's use a fixed secret key for the resolver so we can predict the pointer key.
        
        let resolver_sk = SecretKey::from_hex("55dcbc4624699d219b8ec293339a3b81e68815397f5a502026784d8122d09fce").unwrap();
        let name = "test.name";
        let expected_pointer_key = Client::register_key_from_name(&resolver_sk, name);
        let expected_addr = PointerAddress::from_hex(&expected_pointer_key.public_key().to_hex()).unwrap();

        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_chunk_caching_client = MockChunkCachingClient::default();

        let p1 = Pointer::new(&expected_pointer_key, 0, PointerTarget::PointerAddress(addr2));
        let p1_clone = p1.clone();
        let p2_clone = p2.clone();

        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |addr: &PointerAddress| {
                if addr.to_hex() == expected_addr.to_hex() {
                    Ok(p1_clone.clone())
                } else if addr.to_hex() == addr2.to_hex() {
                    Ok(p2_clone.clone())
                } else {
                    Err(PointerError::GetError(GetError::RecordNotFound("Not found".to_string())))
                }
            });

        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .returning(|_| Err(ChunkError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let _resolver = PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, resolver_sk, 3600);
        let result = _resolver.resolve(&name.to_string()).await;

        assert!(result.is_some());
        assert_eq!(result.unwrap().address, target_chunk_address.to_hex());
    }

    #[tokio::test]
    async fn test_resolve_pointer_cycle() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let sk1 = SecretKey::random();
        let addr1 = PointerAddress::from_hex(&sk1.public_key().to_hex()).unwrap();
        let sk2 = SecretKey::random();
        let addr2 = PointerAddress::from_hex(&sk2.public_key().to_hex()).unwrap();
        
        let p1 = Pointer::new(&sk1, 0, PointerTarget::PointerAddress(addr2));
        let p2 = Pointer::new(&sk2, 0, PointerTarget::PointerAddress(addr1));

        let p1_clone = p1.clone();
        let p2_clone = p2.clone();

        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |addr: &PointerAddress| {
                if addr.to_hex() == addr1.to_hex() {
                    Ok(p1_clone.clone())
                } else {
                    Ok(p2_clone.clone())
                }
            });

        let resolver = PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, sk1, 3600);
        
        // This should hit the 10 iteration limit
        let result = resolver.resolve_pointer(&addr1.to_hex(), 0).await;
        assert!(result.is_err());
    }
}