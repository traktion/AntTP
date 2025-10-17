use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError};
use autonomi::{Client, GraphEntryAddress, PublicKey, SecretKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::graph::{GraphContent};
use hex::FromHex;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::client::error::GraphError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::CacheType;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GraphEntry {
    name: Option<String>,
    content: String,
    #[schema(read_only)]
    address: Option<String>,
    parents: Option<Vec<String>>,
    descendants: Option<Vec<GraphDescendants>>,
    #[schema(read_only)]
    cost: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct GraphDescendants {
    public_key: String,
    content: String,
}

impl GraphEntry {
    pub fn new(name: Option<String>, content: String, address: Option<String>, parents: Option<Vec<String>>, descendants: Option<Vec<GraphDescendants>>, cost: Option<String>) -> Self {
        GraphEntry { name, content, address, parents, descendants, cost }
    }
}

impl GraphDescendants {
    pub fn new(public_key: String, content: String) -> Self {
        GraphDescendants{public_key, content}
    }
}

pub struct GraphService {
    caching_client: CachingClient,
    ant_tp_config: AntTpConfig,
}

impl GraphService {

    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig) -> Self {
        GraphService { caching_client, ant_tp_config }
    }

    pub async fn create_graph_entry(&self, graph: GraphEntry, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<HttpResponse, Error> {
        let app_secret_key = SecretKey::from_hex(self.ant_tp_config.app_private_key.clone().as_str()).unwrap();
        let graph_key = Client::register_key_from_name(&app_secret_key, graph.name.clone().unwrap().as_str());

        let mut graph_parents = vec![];
        let parents = graph.parents.clone();
        if parents.is_some() {
            parents.unwrap().iter().for_each(|p| {
                graph_parents.push(PublicKey::from_hex(p).unwrap());
            });
        }

        let mut graph_descendants = vec![];
        let descendants = graph.descendants.clone();
        if descendants.is_some() {
            descendants.unwrap().iter()
                .for_each(|d| {
                    let key = PublicKey::from_hex(d.clone().public_key.as_str()).unwrap();
                    let content = GraphContent::from_hex(d.clone().content.clone()).unwrap();
                    graph_descendants.push((key, content))
                });
        }

        let graph_content = GraphContent::from_hex(graph.content.clone()).unwrap();
        let graph_entry = autonomi::GraphEntry::new(&graph_key, graph_parents, graph_content.clone(), graph_descendants);
        info!("Create graph entry from name [{}] for content [{}]", graph.name.clone().unwrap(), graph.content.clone());
        match self.caching_client
            .graph_entry_put(graph_entry, PaymentOption::from(&evm_wallet), cache_only)
            .await {
                Ok((cost, graph_entry_address)) => {
                    info!("Created graph entry at [{}] for [{}] attos", graph_entry_address.to_hex(), cost);
                    let response_graph = GraphEntry::new(graph.name, graph.content, Some(graph_entry_address.to_hex()), graph.parents, graph.descendants, Some(cost.to_string()));
                    Ok(HttpResponse::Created().json(response_graph))
                }
                Err(e) => {
                    // todo: refine error handling to return appropriate messages / payloads
                    warn!("Failed to create graph entry: [{:?}]", e);
                    Err(ErrorInternalServerError("Failed to create graph entry"))
                }
        }
    }

    pub async fn get_graph_entry(&self, address: String) -> Result<GraphEntry, GraphError> {
        let graph_entry_address = GraphEntryAddress::from_hex(address.as_str()).unwrap();
        match self.caching_client.graph_entry_get(&graph_entry_address).await {
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

                Ok(GraphEntry::new(None, hex::encode(graph_entry.content.clone()), Some(address), graph_parents, graph_descendants, None))
            }
            Err(e) => {
                warn!("Failed to retrieve graph entry at address [{}]: [{:?}]", address, e);
                Err(e)
            }
        }
    }
}