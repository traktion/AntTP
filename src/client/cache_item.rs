use std::time::{SystemTime, UNIX_EPOCH};

pub struct CacheItem<T> {
    pub item: Option<T>,
    pub ttl: u64,
    pub timestamp: u64
}

impl<T> CacheItem<T> {
    pub fn new(item: Option<T>, ttl: u64) -> Self {
        CacheItem { item, ttl, timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() }
    }
    
    pub fn has_expired(&self) -> bool {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - self.timestamp > self.ttl
    }
}