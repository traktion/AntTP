use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use ant_evm::EvmNetwork::ArbitrumOne;
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
        "traktion-blog=8e16406561d0c460f3dbe37fef129582d6410ec7cb9d5aebdf9cbb051676624c543a315f7e857103cd71088a927c9085,\
        imim=959c2ba5b84e1a68fedc14caaae96e97cfff19ff381127844586b2e0cdd2afdfb1687086a5668bced9f3dc35c03c9bd7,\
        gimim=82fb48d691a65e771e2279ff56d8c5f7bc007fa386c9de95d64be52e081f01b1fdfb248095238b93db820836cc88c67a,\
        index=b970cf40a1ba880ecc27d5495f543af387fcb014863d0286dd2b1518920df38ac311d854013de5d50b9b04b84a6da021,\
        gindex=879d061580e6200a3f1dbfc5c87c13544fcd391dfec772033f1138a9469df35c98429ecd3acb4a9ab631ea7d5f6fae0f,\
        cinema=953ff297c689723a59e20d6f80b67233b0c0fe17ff4cb37a2c8cfb46e276ce0e45d59c17e006e4990deaa634141e4c77"
    )]
    pub bookmarks_vec: Vec<String>,

    #[clap(skip)]
    pub bookmarks_map: HashMap<String, String>,

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
        info!("Bookmarks: {:?}", ant_tp_config.bookmarks_vec);
        info!("Cached mutable TTL: {:?}", ant_tp_config.cached_mutable_ttl);
        info!("Peers: {:?}", ant_tp_config.peers);
        info!("Map cache directory: {:?}", ant_tp_config.map_cache_directory);
        info!("EVM network: {:?}", ant_tp_config.evm_network);
        info!("Immutable disk cache size (MB): {:?}", ant_tp_config.immutable_disk_cache_size);
        info!("Immutable memory cache size (slots): {:?}", ant_tp_config.immutable_memory_cache_size);
        info!("Idle disconnect from Autonomi (seconds): {:?}", ant_tp_config.idle_disconnect);
        info!("Command buffer size (slots): {:?}", ant_tp_config.command_buffer_size);
        ant_tp_config.update_bookmarks_map()
    }

    pub fn get_default_map_cache_directory() -> String {
        env::temp_dir().to_str().unwrap().to_owned() + "/anttp/cache/"
    }

    pub fn get_default_evm_network() -> String {
        ArbitrumOne.to_string()
    }

    pub fn update_bookmarks_map(mut self) -> Self {
        self.bookmarks_map = self.bookmarks_vec.clone().iter()
            .into_iter()
            .map(|s| s.split_at(s.find("=").unwrap()))
            .map(|(key, val)| (key.to_string(), val[1..].to_string()))
            .collect();
        self
        //self.bookmarks.clone().into_iter().map(|data| data.split("=")   ("1".to_string(), "2".to_string()) ).collect()
        //self.bookmarks.iter().filter(|&s| s.starts_with(alias_with_delimiter.as_str())).next().is_some()
    }
}