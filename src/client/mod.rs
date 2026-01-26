use actix_web::web::Data;
use foyer::HybridCache;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use crate::client::client_harness::ClientHarness;
use command::Command;
use crate::config::anttp_config::AntTpConfig;

#[derive(Debug, Clone)]
pub struct CachingClient {
    pub client_harness: Data<Mutex<ClientHarness>>,
    pub ant_tp_config: AntTpConfig,
    pub hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    pub command_executor: Data<Sender<Box<dyn Command>>>,
}

const ARCHIVE_CACHE_KEY: &'static str = "ar";
const GRAPH_ENTRY_CACHE_KEY: &'static str = "gg";
const POINTER_CACHE_KEY: &'static str = "pg";
const POINTER_CHECK_CACHE_KEY: &'static str = "pce";
const PUBLIC_ARCHIVE_CACHE_KEY: &'static str = "pa";
const REGISTER_CACHE_KEY: &'static str = "rg";
const SCRATCHPAD_CACHE_KEY: &'static str = "sg";
const TARCHIVE_CACHE_KEY: &'static str = "tar";

pub mod caching_client;
pub mod cache_item;
pub mod client_harness;
pub mod chunk_caching_client;
pub mod scratchpad_caching_client;
pub mod graph_entry_caching_client;
pub mod pointer_caching_client;
pub mod register_caching_client;
pub mod public_archive_caching_client;
pub mod tarchive_caching_client;
pub mod archive_caching_client;
pub mod public_data_caching_client;
pub mod command;

pub use self::caching_client::*;
pub use chunk_caching_client::{ChunkCachingClient, MockChunkCachingClient};
pub use scratchpad_caching_client::ScratchpadCachingClient;
pub use graph_entry_caching_client::GraphEntryCachingClient;
pub use pointer_caching_client::PointerCachingClient;
pub use register_caching_client::RegisterCachingClient;
pub use public_archive_caching_client::PublicArchiveCachingClient;
pub use tarchive_caching_client::TArchiveCachingClient;
pub use archive_caching_client::ArchiveCachingClient;
pub use public_data_caching_client::PublicDataCachingClient;
