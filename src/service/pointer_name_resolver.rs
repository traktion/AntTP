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
                    Ok(pointer) => {
                        let p: Pointer = pointer;
                        match p.target() {
                            PointerTarget::ChunkAddress(_) => Ok(p),
                            _ => Box::pin(self.resolve_pointer(&p.target().to_hex(), iteration + 1)).await,
                        }
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
                            let p: Pointer = pointer;
                            self.pointer_caching_client.pointer_update_ttl(&p.address(), ttl_override).await?;
                            match p.target() {
                                PointerTarget::ChunkAddress(_) => Ok(p),
                                _ => Box::pin(self.update_pointer_ttls(&p.target().to_hex(), ttl_override, iteration + 1)).await,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::chunk_caching_client::MockChunkCachingClient;
    use crate::client::pointer_caching_client::MockPointerCachingClient;
    use crate::error::chunk_error::ChunkError;
    use crate::model::pnr::{PnrRecord, PnrRecordType, PnrZone};
    use autonomi::{Chunk, ChunkAddress, PointerAddress, SecretKey};
    use mockall::predicate::*;

    fn create_test_resolver(
        mock_pointer_caching_client: MockPointerCachingClient,
        mock_chunk_caching_client: MockChunkCachingClient,
    ) -> PointerNameResolver {
        let secret_key = SecretKey::random();
        PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, secret_key, 3600)
    }

    #[tokio::test]
    async fn test_is_resolved_success() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let name = "test.ant".to_string();
        let secret_key = SecretKey::random();
        let pointer_key = Client::register_key_from_name(&secret_key, name.as_str());
        let pointer_address = PointerAddress::from_hex(pointer_key.public_key().to_hex().as_str()).unwrap();
        let target_chunk = ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap();
        let pointer = Pointer::new(&pointer_key, 0, PointerTarget::ChunkAddress(target_chunk));

        mock_pointer_caching_client
            .expect_pointer_get()
            .with(eq(pointer_address))
            .times(1)
            .returning(move |_| Ok(pointer.clone()));
        
        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .returning(|_| Err(ChunkError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let resolver = PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, secret_key, 3600);
        assert!(resolver.is_resolved(&name).await);
    }

    #[tokio::test]
    async fn test_resolve_pointer_not_found() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(|_| Err(PointerError::GetError(GetError::RecordNotFound("Not found".to_string()))));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve_pointer(&"0000000000000000000000000000000000000000000000000000000000000000".to_string(), 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_map_success() {
        let mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let chunk_address_hex = "a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527".to_string();
        let chunk_address = ChunkAddress::from_hex(&chunk_address_hex).unwrap();
        
        let pnr_zone = PnrZone::new(
            "test.ant".to_string(),
            vec![PnrRecord::new(None, "target_address".to_string(), PnrRecordType::A, 300)],
            None,
            None
        );
        let chunk_data = serde_json::to_vec(&pnr_zone).unwrap();
        let chunk = Chunk::new(bytes::Bytes::from(chunk_data));

        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .with(eq(chunk_address))
            .times(1)
            .returning(move |_| Ok(chunk.clone()));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve_map(&chunk_address_hex).await;
        
        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.address, "target_address");
        assert_eq!(record.ttl, 300);
    }

    #[tokio::test]
    async fn test_resolve_pointer_cyclic_loop() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let sk1 = SecretKey::random();
        let sk2 = SecretKey::random();
        
        let addr1 = PointerAddress::from_hex(sk1.public_key().to_hex().as_str()).unwrap();
        let addr2 = PointerAddress::from_hex(sk2.public_key().to_hex().as_str()).unwrap();
        
        let p1 = Pointer::new(&sk1, 0, PointerTarget::PointerAddress(addr2.clone()));
        let p2 = Pointer::new(&sk2, 0, PointerTarget::PointerAddress(addr1.clone()));

        let addr1_hex = addr1.to_hex();
        let addr1_hex_clone = addr1_hex.clone();
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |addr| {
                if addr.to_hex() == addr1_hex_clone {
                    Ok(p1.clone())
                } else {
                    Ok(p2.clone())
                }
            });

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve_pointer(&addr1_hex, 0).await;
        assert!(result.is_err());
        if let Err(PointerError::GetError(GetError::RecordNotFound(msg))) = result {
            assert!(msg.contains("Too many iterations"));
        } else {
            panic!("Expected loop error");
        }
    }

    #[tokio::test]
    async fn test_update_pointer_ttls_success() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let sk = SecretKey::random();
        let addr = PointerAddress::from_hex(sk.public_key().to_hex().as_str()).unwrap();
        let target_chunk = ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap();
        let pointer = Pointer::new(&sk, 0, PointerTarget::ChunkAddress(target_chunk));

        let addr_hex = addr.to_hex();
        let pointer_clone = pointer.clone();
        mock_pointer_caching_client
            .expect_pointer_get()
            .with(eq(addr))
            .times(1)
            .returning(move |_| Ok(pointer_clone.clone()));

        mock_pointer_caching_client
            .expect_pointer_update_ttl()
            .with(eq(addr), eq(600))
            .times(1)
            .returning(move |_, _| Ok(pointer.clone()));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.update_pointer_ttls(&addr_hex, 600, 0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_success_with_map() {
        let mut mock_pointer_caching_client = MockPointerCachingClient::default();
        let mut mock_chunk_caching_client = MockChunkCachingClient::default();
        
        let name = "test.ant".to_string();
        let resolver_sk = SecretKey::random();
        let pointer_key = Client::register_key_from_name(&resolver_sk, name.as_str());
        let pointer_address = PointerAddress::from_hex(pointer_key.public_key().to_hex().as_str()).unwrap();
        let target_chunk_addr = ChunkAddress::from_hex("a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527").unwrap();
        let pointer = Pointer::new(&pointer_key, 0, PointerTarget::ChunkAddress(target_chunk_addr));

        // resolve() calls resolve_pointer()
        let pointer_clone = pointer.clone();
        mock_pointer_caching_client
            .expect_pointer_get()
            .returning(move |_| Ok(pointer_clone.clone()));

        // resolve() calls resolve_map() if target is a chunk
        let pnr_zone = PnrZone::new(
            "test.ant".to_string(),
            vec![PnrRecord::new(None, "final_target".to_string(), PnrRecordType::A, 300)],
            None,
            None
        );
        let chunk_data = serde_json::to_vec(&pnr_zone).unwrap();
        let chunk = Chunk::new(bytes::Bytes::from(chunk_data));
        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .returning(move |_| Ok(chunk.clone()));

        // update_pointer_ttls is called after successful map resolution
        mock_pointer_caching_client
            .expect_pointer_update_ttl()
            .returning(move |_, _| Ok(pointer.clone()));

        let resolver = PointerNameResolver::new(mock_pointer_caching_client, mock_chunk_caching_client, resolver_sk, 3600);
        let result = resolver.resolve(&name).await;
        
        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.address, "final_target");
        assert_eq!(record.ttl, 300);
    }
}