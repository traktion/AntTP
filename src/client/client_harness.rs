use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::web::Data;
use ant_core::data::{Client, ClientConfig, Error};
use evmlib::Network;
use evmlib::wallet::Wallet;
use log::{debug, info};
use crate::config::anttp_config::AntTpConfig;

pub struct ClientHarness {
    network: Network,
    ant_tp_config: AntTpConfig,
    maybe_client: Option<Data<Client>>,
    last_accessed_time: u64,
}

impl ClientHarness {
    pub fn new(network: Network, ant_tp_config: AntTpConfig) -> Self {
        let last_accessed_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        ClientHarness { network, ant_tp_config, maybe_client: None, last_accessed_time }
    }

    pub async fn get_client(&mut self) -> Result<Data<Client>, Error> {
        self.last_accessed_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if self.maybe_client.is_none() {
            self.maybe_client = Some(Data::new(self.init_client().await?));
        }
        match self.maybe_client.clone() {
            Some(client) => Ok(client),
            None => Err(Error::Network("Client failure".to_string()))
        }
    }

    pub fn try_sleep(&mut self) {
        // if idle for a period, deallocate the client to save resources (CPU/memory)
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if self.maybe_client.is_some() {
            if current_time - self.last_accessed_time > self.ant_tp_config.idle_disconnect {
                info!("idle timeout exceeded... disconnecting from network");
                self.maybe_client = None;
            } else {
                debug!("staying awake... current_time: {}, last_accessed_time: {}", current_time, self.last_accessed_time);
            }
        }
    }

    async fn init_client(&self) -> Result<Client, Error> {
        let bootstrap_peers = match ant_core::config::load_bootstrap_peers() {
            Ok(Some(bootstrap_peers)) => bootstrap_peers,
            Ok(None) => return Err(Error::Network("failed to find bootstrap peers".to_string())),
            Err(e) => return Err(Error::Network(format!("failed to load bootstrap peers: {}", e))),
        };

        let wallet = match Wallet::new_from_private_key(self.network.clone(), self.ant_tp_config.wallet_private_key.as_str()) {
            Ok(wallet) => wallet,
            Err(e) => return Err(Error::Network(format!("failed to load wallet: {}", e))),
        };

        Ok(
            Client::connect(
                bootstrap_peers.as_slice(),
                ClientConfig::default(),
            ).await?
                .with_evm_network(self.network.clone())
                .with_wallet(wallet)
                //todo: disable chunk cache, as foyer provides this (and with more control)
        )
    }
}