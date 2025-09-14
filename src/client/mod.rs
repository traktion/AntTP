use actix_web::web::Data;
use foyer::HybridCache;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use crate::config::anttp_config::AntTpConfig;

#[derive(Clone)]
pub struct CachingClient {
    client_harness: Data<Mutex<ClientHarness>>,
    ant_tp_config: AntTpConfig,
    hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
}

pub mod caching_client;
pub mod cache_item;
pub mod client_harness;
pub mod chunk_caching_client;
mod scratchpad_caching_client;
mod graph_entry_caching_client;
mod pointer_caching_client;
mod register_caching_client;
mod public_archive_caching_client;
mod tarchive_caching_client;
mod archive_caching_client;
mod public_caching_client;