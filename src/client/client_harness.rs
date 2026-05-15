use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::web::Data;
use ant_core::data::{Client, ClientConfig, Error};
use evmlib::Network;
use log::{debug, info};
use crate::config::anttp_config::AntTpConfig;

pub struct ClientHarness {
    evm_network: Network,
    ant_tp_config: AntTpConfig,
    maybe_client: Option<Data<Client>>,
    last_accessed_time: u64,
}

impl ClientHarness {
    pub fn new(evm_network: Network, ant_tp_config: AntTpConfig) -> Self {
        let last_accessed_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        ClientHarness { evm_network, ant_tp_config, maybe_client: None, last_accessed_time }
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
        // todo: fix unwraps
        Ok(
            Client::connect(
                ant_core::config::load_bootstrap_peers().unwrap().unwrap().as_slice(),
                ClientConfig::default(),
            ).await?
                //.with_wallet(Wallet::new_from_private_key(EvmNetwork::ArbitrumOne, self.ant_tp_config.wallet_private_key.as_str()).unwrap())
                .with_evm_network(self.evm_network.clone())
        )
        /*Ok(Client {
            config: ClientConfig::default(),
            network: Network::new(self.ant_tp_config.peers.as_slice(), true).await?,
            wallet: Some(Arc::new(Wallet::new_with_random_wallet(EvmNetwork::ArbitrumOne))),
            evm_network: Some(self.evm_network.clone()),
            chunk_cache: ChunkCache::new(0),
            next_request_id: AtomicU64::new(0),
        })*/
    }
}