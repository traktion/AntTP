use std::net::SocketAddr;
use autonomi::SecretKey;
use log::info;
use clap::Parser;

#[derive(Clone, Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AntTpConfig {
    #[arg(short, long, default_value = "0.0.0.0:8080")]
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
        "traktion-blog=b70a146f95b3ff237fa8140c4175f6a302c8250fe268aacdb47c2783f2b2ee6af5575410d07f6eae9ac7fb9ce95995e4,imim=959c2ba5b84e1a68fedc14caaae96e97cfff19ff381127844586b2e0cdd2afdfb1687086a5668bced9f3dc35c03c9bd7"
    )]
    pub bookmarks: Vec<String>,

    #[arg(short, long)]
    pub uploads_disabled: bool,
}

impl AntTpConfig {

    pub fn read_args() -> AntTpConfig {
        let ant_to_config = AntTpConfig::parse();
        info!("Listen address [{}]", ant_to_config.listen_address);
        info!("Static file directory: [{}]", ant_to_config.static_file_directory);
        info!("Wallet private key: [*****]");
        info!("Download threads: [{}]", ant_to_config.download_threads);
        info!("Uploads disabled: [{}]", ant_to_config.uploads_disabled);
        if ant_to_config.app_private_key.is_empty() {
            info!("No app private key provided. Try this one: [{:?}]", SecretKey::random().to_hex());
        } else {
            info!("App private key: [*****]");
        }
        info!("Bookmarks {:?}", ant_to_config.bookmarks);
        ant_to_config
    }
}