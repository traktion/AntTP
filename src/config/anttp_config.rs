use std::env;
use std::net::SocketAddr;
use ant_evm::EvmNetwork::ArbitrumOne;
use autonomi::{Multiaddr, SecretKey};
use log::info;
use clap::Parser;
use crate::error::CreateError;

#[derive(Clone, Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AntTpConfig {
    #[arg(short, long, default_value = "0.0.0.0:18888")]
    pub listen_address: SocketAddr,

    #[arg(long, default_value = "0.0.0.0:18889")]
    pub https_listen_address: SocketAddr,

    #[arg(short, long, default_value = "")]
    pub static_file_directory: String,

    #[arg(short, long, default_value = "")]
    pub wallet_private_key: String,

    #[arg(short, long, default_value_t = 8)]
    pub download_threads: usize,

    #[arg(short, long, default_value = "")]
    pub app_private_key: String,

    #[arg(short, long, default_value = "55dcbc4624699d219b8ec293339a3b81e68815397f5a502026784d8122d09fce")]
    pub resolver_private_key: String,

    #[arg(short, long, default_value = "a40e045a6fbed33b27039aa8383c9dbf286e19a7265141c2da3085e0c8571527a73a699f9f96a80045391a23356ed0e3")]
    pub bookmarks_address: String,

    #[arg(short, long, default_value_t = false)]
    pub uploads_disabled: bool,

    #[arg(short, long, default_value_t = 5)]
    pub cached_mutable_ttl: u64,

    #[arg(short, long, value_delimiter = ',')]
    pub peers: Vec<Multiaddr>,

    #[arg(short, long, default_value_t = AntTpConfig::get_default_map_cache_directory())]
    pub map_cache_directory: String,

    #[arg(short, long, default_value_t = AntTpConfig::get_default_evm_network())]
    pub evm_network: String,

    #[arg(long, default_value_t = 1024)]
    pub immutable_disk_cache_size: usize,

    #[arg(long, default_value_t = 32)]
    pub immutable_memory_cache_size: usize,

    #[arg(short, long, default_value_t = 30)]
    pub idle_disconnect: u64,

    #[arg(long, default_value_t = 128)]
    pub command_buffer_size: usize,

    #[arg(long, default_value = "")]
    pub access_list_address: String,
}

impl AntTpConfig {

    pub fn read_args() -> AntTpConfig {
        let ant_tp_config = AntTpConfig::parse();
        info!("Listen address: [{}]", ant_tp_config.listen_address);
        info!("Static file directory: [{}]", ant_tp_config.static_file_directory);
        info!("Wallet private key: [*****]");
        info!("Download threads: [{}]", ant_tp_config.download_threads);
        info!("Uploads disabled: [{}]", ant_tp_config.uploads_disabled);
        if ant_tp_config.app_private_key.is_empty() {
            info!("No app/personal private key provided. Try this one: [{:?}]", SecretKey::random().to_hex());
        } else {
            info!("App/personal private key: [*****]");
        }
        info!("Bookmarks address: {:?}", ant_tp_config.bookmarks_address);
        info!("Cached mutable TTL: {:?}", ant_tp_config.cached_mutable_ttl);
        info!("Peers: {:?}", ant_tp_config.peers);
        info!("Map cache directory: {:?}", ant_tp_config.map_cache_directory);
        info!("EVM network: {:?}", ant_tp_config.evm_network);
        info!("Immutable disk cache size (MB): {:?}", ant_tp_config.immutable_disk_cache_size);
        info!("Immutable memory cache size (slots): {:?}", ant_tp_config.immutable_memory_cache_size);
        info!("Idle disconnect from Autonomi (seconds): {:?}", ant_tp_config.idle_disconnect);
        info!("Command buffer size (slots): {:?}", ant_tp_config.command_buffer_size);
        info!("Access list archive: {:?}", ant_tp_config.access_list_address);
        info!("Resolver private key: {:?}", ant_tp_config.resolver_private_key);
        ant_tp_config
    }

    pub fn get_default_map_cache_directory() -> String {
        env::temp_dir().to_str().unwrap().to_owned() + "/anttp/cache/"
    }

    pub fn get_default_evm_network() -> String {
        ArbitrumOne.to_string()
    }

    pub fn get_app_private_key(&self) -> Result<SecretKey, CreateError> {
        match SecretKey::from_hex(self.app_private_key.clone().as_str()) {
            Ok(app_secret_key) => Ok(app_secret_key),
            Err(e) => Err(CreateError::DataKeyMissing(e.to_string()))
        }
    }

    pub fn get_resolver_private_key(&self) -> Result<SecretKey, CreateError> {
        match SecretKey::from_hex(self.resolver_private_key.clone().as_str()) {
            Ok(resolver_secret_key) => Ok(resolver_secret_key),
            Err(e) => Err(CreateError::DataKeyMissing(e.to_string()))
        }
    }
}