use ant_protocol::storage::{ChunkAddress, Pointer, PointerAddress, PointerTarget};
use autonomi::{Client, SecretKey};
use log::{debug, error, warn};
use std::collections::HashSet;
use once_cell::sync::Lazy;
#[double]
use crate::client::ChunkCachingClient;
#[double]
use crate::client::PointerCachingClient;
use crate::error::GetError;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::PnrZone;
use mockall_double::double;

static PUBLIC_SUFFIX_LIST: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    // TLDs (representative sample)
    s.insert("com");
    s.insert("org");
    s.insert("net");
    s.insert("edu");
    s.insert("gov");
    s.insert("io");
    s.insert("uk");
    s.insert("de");
    s.insert("jp");
    s.insert("fr");
    s.insert("au");
    s.insert("ca");
    s.insert("it");
    s.insert("ch");
    s.insert("nl");
    s.insert("no");
    s.insert("se");
    s.insert("es");
    s.insert("pt");
    s.insert("gr");
    s.insert("ru");
    s.insert("cn");
    s.insert("in");
    s.insert("br");
    s.insert("za");
    s.insert("mx");
    s.insert("ar");
    s.insert("cl");
    s.insert("co");
    s.insert("pe");
    s.insert("ve");
    s.insert("ec");
    s.insert("uy");
    s.insert("py");
    s.insert("bo");
    s.insert("gy");
    s.insert("sr");
    s.insert("gf");

    // SLDs (representative sample)
    s.insert("co.uk");
    s.insert("org.uk");
    s.insert("me.uk");
    s.insert("com.au");
    s.insert("org.au");
    s.insert("com.br");
    s.insert("org.br");
    s.insert("com.cn");
    s.insert("org.cn");
    s.insert("net.cn");
    s.insert("ac.uk");
    s.insert("gov.uk");
    s.insert("ltd.uk");
    s.insert("plc.uk");
    s.insert("sch.uk");
    s.insert("co.jp");
    s.insert("or.jp");
    s.insert("ne.jp");
    s.insert("ac.jp");
    s.insert("ad.jp");
    s.insert("ed.jp");
    s.insert("go.jp");
    s.insert("gr.jp");
    s.insert("lg.jp");

    s
});

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

    fn validate_pnr_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 63 {
            return false;
        }
        if name.starts_with('-') || name.ends_with('-') {
            return false;
        }
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    }

    fn split_name(name: &str) -> (String, String) {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() <= 1 {
            return (name.to_string(), "".to_string());
        }

        // Check for known SLDs (2 parts)
        if parts.len() >= 3 {
            let last_two = format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
            if PUBLIC_SUFFIX_LIST.contains(last_two.as_str()) {
                let zone_name = format!("{}.{}", parts[parts.len() - 3], last_two);
                let sub_name = parts[..parts.len() - 3].join(".");
                return (zone_name, sub_name);
            }
        }

        // Check for known TLDs (1 part)
        if parts.len() >= 2 {
            let last_one = parts[parts.len() - 1];
            if PUBLIC_SUFFIX_LIST.contains(last_one) {
                let zone_name = format!("{}.{}", parts[parts.len() - 2], last_one);
                let sub_name = parts[..parts.len() - 2].join(".");
                return (zone_name, sub_name);
            }
        }

        // Special case: "test.name" in tests is often used as a single part zone name
        // or the second part is not in our restricted sample TLD list.
        // If we have "test.name", we might want "test.name" to be the zone name if it's treated as a single PNR name.
        // But traditional PNR names might not have dots unless they are sub-names.
        
        // If the last part is not a known TLD/SLD, we assume the last part IS the zone name.
        let zone_name = parts[parts.len() - 1].to_string();
        let sub_name = parts[..parts.len() - 1].join(".");
        (zone_name, sub_name)
    }

    pub async fn resolve(&self, name: &String) -> Option<ResolvedRecord> {
        if !Self::validate_pnr_name(name) {
            return None;
        }

        let (zone_name, sub_name) = Self::split_name(name);
        debug!("resolve: name={}, zone_name={}, sub_name={}", name, zone_name, sub_name);

        let pointer_key = Client::register_key_from_name(&self.pointer_name_resolver_secret_key, zone_name.as_str());
        debug!("found: zone_name={}, pointer_key={}, public_key={}", zone_name, pointer_key.to_hex(), &pointer_key.public_key().to_hex());

        match self.resolve_pointer(&pointer_key.public_key().to_hex(), 0).await.ok() {
            Some(pointer) => match self.resolve_map(&pointer.target().to_hex(), &sub_name).await {
                Some(resolved_record) => {
                    // use resolved record TTL to update TTLs for cache
                    self.update_pointer_ttls(&pointer_key.public_key().to_hex(), resolved_record.ttl, 0).await.ok()?;
                    Some(resolved_record) // return map target
                },
                None => {
                    if sub_name.is_empty() {
                        Some(ResolvedRecord::new(pointer.target().to_hex(), self.ttl_default)) // return pointer target if no sub-name
                    } else {
                        None
                    }
                }
            }
            None => None,
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

    pub async fn resolve_map(&self, name: &String, sub_name: &String) -> Option<ResolvedRecord> {
        match ChunkAddress::from_hex(&name) {
            Ok(chunk_address) => match self.chunk_caching_client.chunk_get_internal(&chunk_address).await {
                Ok(chunk) => {
                    match serde_json::from_slice::<PnrZone>(&chunk.value) {
                        Ok(pnr_zone) => {
                            debug!("deserialized {} PNR records", pnr_zone.records.len());
                            if let Some(pnr_record) = pnr_zone.records.get(sub_name) {
                                debug!("found PNR record for sub_name: '{}'", sub_name);
                                return Some(ResolvedRecord::new(pnr_record.address.to_string(), pnr_record.ttl));
                            }
                            debug!("no PNR record found for sub_name: '{}'", sub_name);
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
        let result = resolver.resolve(&"testname".to_string()).await;

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
        let result = resolver.resolve(&"testname".to_string()).await;

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.address, target_chunk_address.to_hex());
        assert_eq!(record.ttl, 3600);
    }

    #[tokio::test]
    async fn test_validate_pnr_name() {
        assert!(PointerNameResolver::validate_pnr_name("test"));
        assert!(PointerNameResolver::validate_pnr_name("test-name"));
        assert!(PointerNameResolver::validate_pnr_name("sub.test-name"));
        assert!(PointerNameResolver::validate_pnr_name("a".repeat(63).as_str()));
        
        assert!(!PointerNameResolver::validate_pnr_name(""));
        assert!(!PointerNameResolver::validate_pnr_name("-test"));
        assert!(!PointerNameResolver::validate_pnr_name("test-"));
        assert!(!PointerNameResolver::validate_pnr_name("a".repeat(64).as_str()));
        assert!(!PointerNameResolver::validate_pnr_name("test_name"));
    }

    #[test]
    fn test_split_name() {
        let (zone, sub) = PointerNameResolver::split_name("sub.zone");
        assert_eq!(zone, "zone");
        assert_eq!(sub, "sub");

        let (zone, sub) = PointerNameResolver::split_name("sub1.sub2.zone");
        assert_eq!(zone, "zone");
        assert_eq!(sub, "sub1.sub2");

        let (zone, sub) = PointerNameResolver::split_name("zone.com");
        assert_eq!(zone, "zone.com");
        assert_eq!(sub, "");

        let (zone, sub) = PointerNameResolver::split_name("sub.zone.com");
        assert_eq!(zone, "zone.com");
        assert_eq!(sub, "sub");

        let (zone, sub) = PointerNameResolver::split_name("sub1.sub2.zone.com");
        assert_eq!(zone, "zone.com");
        assert_eq!(sub, "sub1.sub2");

        let (zone, sub) = PointerNameResolver::split_name("zone.co.uk");
        assert_eq!(zone, "zone.co.uk");
        assert_eq!(sub, "");

        let (zone, sub) = PointerNameResolver::split_name("sub.zone.co.uk");
        assert_eq!(zone, "zone.co.uk");
        assert_eq!(sub, "sub");
    }

    #[tokio::test]
    async fn test_resolve_with_subname() {
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

        let mut records = HashMap::new();
        records.insert("sub".to_string(), PnrRecord::new("address123".to_string(), PnrRecordType::A, 100));
        let pnr_zone = PnrZone::new("testname".to_string(), records, None, None);
        let pnr_zone_bytes = serde_json::to_vec(&pnr_zone).unwrap();

        mock_chunk_caching_client
            .expect_chunk_get_internal()
            .returning(move |_| Ok(Chunk::new(Bytes::from(pnr_zone_bytes.clone()))));

        let resolver = create_test_resolver(mock_pointer_caching_client, mock_chunk_caching_client);
        let result = resolver.resolve(&"sub.testname".to_string()).await;

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.address, "address123");
        assert_eq!(record.ttl, 100);
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
        let result = resolver.resolve(&"testname".to_string()).await;

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
        let name = "testname";
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