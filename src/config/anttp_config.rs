use std::net::SocketAddr;
use autonomi::{Multiaddr, SecretKey};
use log::info;
use clap::Parser;

#[derive(Clone, Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AntTpConfig {
    #[arg(short, long, default_value = "0.0.0.0:18888")]
    pub listen_address: SocketAddr,

    #[arg(short, long, default_value = "")]
    pub static_file_directory: String,

    #[arg(short, long, default_value = "")]
    pub wallet_private_key: String,

    #[arg(short, long, default_value_t = 8)]
    pub download_threads: usize,

    #[arg(short, long, default_value = "")]
    pub app_private_key: String,

    #[arg(short, long, value_delimiter = ',', default_value =
        "traktion-blog=b70a146f95b3ff237fa8140c4175f6a302c8250fe268aacdb47c2783f2b2ee6af5575410d07f6eae9ac7fb9ce95995e4,\
        imim=959c2ba5b84e1a68fedc14caaae96e97cfff19ff381127844586b2e0cdd2afdfb1687086a5668bced9f3dc35c03c9bd7,\
        index=b970cf40a1ba880ecc27d5495f543af387fcb014863d0286dd2b1518920df38ac311d854013de5d50b9b04b84a6da021"
    )]
    pub bookmarks: Vec<String>,

    #[arg(short, long)]
    pub uploads_disabled: bool,

    #[arg(short, long, default_value_t = 60)]
    pub cached_mutable_ttl: u64,

    #[arg(short, long, value_delimiter = ',')]
    pub peers: Vec<Multiaddr>,

    #[arg(short, long, default_value = "")]
    pub map_cache_directory: String,

    #[arg(short, long, default_value = "")]
    pub evm_network: String,

    #[arg(long, default_value_t = 1024)]
    pub immutable_disk_cache_size: usize,

    #[arg(long, default_value_t = 32)]
    pub immutable_memory_cache_size: usize
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
            info!("No app private key provided. Try this one: [{:?}]", SecretKey::random().to_hex());
        } else {
            info!("App private key: [*****]");
        }
        info!("Bookmarks: {:?}", ant_tp_config.bookmarks);
        info!("Cached mutable TTL: {:?}", ant_tp_config.cached_mutable_ttl);
        info!("Peers: {:?}", ant_tp_config.peers);
        info!("Map cache directory: {:?}", ant_tp_config.map_cache_directory);
        info!("EVM network: {:?}", ant_tp_config.evm_network);
        info!("Immutable disk cache size (MB): {:?}", ant_tp_config.immutable_disk_cache_size);
        info!("Immutable memory cache size (slots): {:?}", ant_tp_config.immutable_memory_cache_size);
        ant_tp_config
    }
}