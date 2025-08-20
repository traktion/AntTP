use std::time::{SystemTime, UNIX_EPOCH};
use ant_evm::EvmNetwork;
use autonomi::{BootstrapCacheConfig, Client, ClientConfig, ClientOperatingStrategy, InitialPeersConfig};
use log::{debug, info, warn};
use crate::config::anttp_config::AntTpConfig;

#[derive(Clone)]
pub struct ClientHarness {
    evm_network: EvmNetwork,
    ant_tp_config: AntTpConfig,
    maybe_client: Option<Client>,
    last_accessed_time: u64,
}

impl ClientHarness {
    pub fn new(evm_network: EvmNetwork, ant_tp_config: AntTpConfig) -> Self {
        let last_accessed_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        ClientHarness { evm_network, ant_tp_config, maybe_client: None, last_accessed_time: last_accessed_time }
    }

    pub async fn get_client(&mut self) -> Option<Client> {
        self.last_accessed_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if self.maybe_client.is_none() {
            self.maybe_client = self.init_client().await;
        };
        self.maybe_client.clone()
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

    async fn init_client(&self) -> Option<Client> {
        let bootstrap_cache_config = Some(BootstrapCacheConfig::new(false).unwrap());

        let initial_peers_config = if self.ant_tp_config.peers.clone().is_empty() {
            InitialPeersConfig::default()
        } else {
            InitialPeersConfig {
                first: false,
                addrs: self.ant_tp_config.peers.clone(),
                network_contacts_url: vec![],
                local: true,
                ignore_cache: false,
                bootstrap_cache_dir: Some(bootstrap_cache_config.clone().unwrap().cache_dir),
            }
        };

        let mut strategy = ClientOperatingStrategy::default();
        strategy.chunk_cache_enabled = false; // disable cache to avoid double-caching

        match Client::init_with_config(ClientConfig {
            bootstrap_cache_config: bootstrap_cache_config.clone(),
            init_peers_config: initial_peers_config,
            evm_network: self.evm_network.clone(),
            strategy: strategy,
            network_id: Some(1),
        }).await {
            Ok(client) => {
                Some(client)
            },
            Err(e) => {
                warn!("Failed to connect to Autonomi Network with error [{}]. Running in offline mode.", e);
                None
            },
        }
    }
}