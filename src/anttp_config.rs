use std::net::SocketAddr;
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
}

impl AntTpConfig {

    pub fn read_args() -> AntTpConfig {
        let ant_to_config = AntTpConfig::parse();
        info!("Listen address [{}]", ant_to_config.listen_address);
        info!("Static file directory: [{}]", ant_to_config.static_file_directory);
        info!("Wallet private key: [*****]");
        info!("Download threads: [{}]", ant_to_config.download_threads);
        ant_to_config
    }
}