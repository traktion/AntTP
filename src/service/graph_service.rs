use autonomi::{Client, GraphEntryAddress, PublicKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::graph::{GraphContent};
use hex::FromHex;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::GraphEntryCachingClient;
use crate::error::graph_error::GraphError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::StoreType;
use crate::error::CreateError;

#[derive(Serialize, Deserialize, ToSchema, Debug, PartialEq)]
pub struct GraphEntry {
    pub name: Option<String>,
    pub content: String,
    #[schema(read_only)]
    pub address: Option<String>,
    pub parents: Option<Vec<String>>,
    pub descendants: Option<Vec<GraphDescendants>>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct GraphDescendants {
    pub public_key: String,
    pub content: String,
}

impl GraphEntry {
    pub fn new(name: Option<String>, content: String, address: Option<String>, parents: Option<Vec<String>>, descendants: Option<Vec<GraphDescendants>>) -> Self {
        GraphEntry { name, content, address, parents, descendants }
    }
}

impl GraphDescendants {
    pub fn new(public_key: String, content: String) -> Self {
        GraphDescendants{public_key, content}
    }
}

#[derive(Debug)]
pub struct GraphService {
    graph_entry_caching_client: GraphEntryCachingClient,
    ant_tp_config: AntTpConfig,
}

impl GraphService {

    pub fn new(graph_entry_caching_client: GraphEntryCachingClient, ant_tp_config: AntTpConfig) -> Self {
        GraphService { graph_entry_caching_client, ant_tp_config }
    }

    pub async fn create_graph_entry(&self, graph: GraphEntry, evm_wallet: Wallet, store_type: StoreType) -> Result<GraphEntry, GraphError> {
        match graph.name {
            Some(name) => {
                let app_secret_key = self.ant_tp_config.get_app_private_key()?;
                let graph_key = Client::register_key_from_name(&app_secret_key, name.as_str());

                let mut data_errors = vec![];
                let mut graph_parents = vec![];
                graph.parents.clone().unwrap_or(vec![]).iter()
                    .for_each(|p| {
                        match PublicKey::from_hex(p) {
                            Ok(public_key) => graph_parents.push(public_key),
                            Err(_) => data_errors.push(format!("parent is not a public key: {}", p))
                        }
                    });

                let mut graph_descendants = vec![];
                graph.descendants.clone().unwrap_or(vec![]).iter()
                    .for_each(|d| {
                        match PublicKey::from_hex(d.public_key.as_str()) {
                            Ok(key) => {
                                match GraphContent::from_hex(d.content.clone()) {
                                    Ok(content) => graph_descendants.push((key, content)),
                                    Err(_) => data_errors.push(format!("content is not a public key: {}", d.content))
                                }
                            }
                            Err(_) => data_errors.push(format!("public_key is not a public key: {}", d.content))
                        }
                    });

                if data_errors.is_empty() {
                    let graph_content = GraphContent::from_hex(graph.content.clone())?;
                    let graph_entry = autonomi::GraphEntry::new(&graph_key, graph_parents, graph_content, graph_descendants);
                    info!("Create graph entry from name [{}] for content [{}]", name, graph.content.clone());
                    let graph_entry_address = self.graph_entry_caching_client
                        .graph_entry_put(graph_entry, PaymentOption::from(&evm_wallet), store_type)
                        .await?;
                    info!("Queued command to create graph entry at [{}]", graph_entry_address.to_hex());
                    Ok(GraphEntry::new(Some(name), graph.content, Some(graph_entry_address.to_hex()), graph.parents, graph.descendants))
                } else {
                    Err(GraphError::CreateError(CreateError::InvalidData(format!("Invalid payload: [{}]", data_errors.join(", ")))))
                }
            },
            None => Err(GraphError::CreateError(CreateError::InvalidData("Name must be provided".to_string())))
        }
    }

    pub async fn get_graph_entry(&self, address: String) -> Result<GraphEntry, GraphError> {
        let graph_entry_address = GraphEntryAddress::from_hex(address.as_str())?;
        match self.graph_entry_caching_client.graph_entry_get(&graph_entry_address).await {
            Ok(graph_entry) => {
                info!("Retrieved graph entry at address [{}] value [{}]", address, hex::encode(graph_entry.content.clone()));

                let graph_parents = if !graph_entry.parents.is_empty() {
                    let mut graph_parents_vec = vec![];
                    graph_entry.parents.iter().for_each(|p| {
                        graph_parents_vec.push(p.to_hex());
                    });
                    Some(graph_parents_vec)
                } else {
                    None
                };

                let graph_descendants = if !graph_entry.descendants.is_empty() {
                    let mut graph_descendants_vec = vec![];
                    graph_entry.descendants.iter()
                        .for_each(|(p, c)| {
                            graph_descendants_vec.push(GraphDescendants::new(p.to_hex(), hex::encode(c)))
                        });
                    Some(graph_descendants_vec)
                } else {
                    None
                };

                Ok(GraphEntry::new(None, hex::encode(graph_entry.content.clone()), Some(address), graph_parents, graph_descendants))
            }
            Err(e) => {
                warn!("Failed to retrieve graph entry at address [{}]: [{:?}]", address, e);
                Err(e)
            }
        }
    }
}
