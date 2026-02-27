use autonomi::{Client, GraphEntryAddress, PublicKey, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::graph::{GraphContent};
use hex::FromHex;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use mockall_double::double;
#[double]
use crate::service::resolver_service::ResolverService;
#[double]
use crate::client::graph_entry_caching_client::GraphEntryCachingClient;
use crate::error::graph_error::GraphError;
use crate::config::anttp_config::AntTpConfig;
use crate::controller::{DataKey, StoreType};
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
    pub key: String,
    pub content: String,
}

impl GraphEntry {
    pub fn new(name: Option<String>, content: String, address: Option<String>, parents: Option<Vec<String>>, descendants: Option<Vec<GraphDescendants>>) -> Self {
        GraphEntry { name, content, address, parents, descendants }
    }
}

impl GraphDescendants {
    pub fn new(key: String, content: String) -> Self {
        GraphDescendants{key, content}
    }
}

#[derive(Debug)]
pub struct GraphService {
    graph_entry_caching_client: GraphEntryCachingClient,
    ant_tp_config: AntTpConfig,
    resolver_service: ResolverService,
}

impl GraphService {

    pub fn new(graph_entry_caching_client: GraphEntryCachingClient, ant_tp_config: AntTpConfig, resolver_service: ResolverService) -> Self {
        GraphService { graph_entry_caching_client, ant_tp_config, resolver_service }
    }

    pub async fn create_graph_entry(&self, graph: GraphEntry, evm_wallet: Wallet, store_type: StoreType, data_key: DataKey) -> Result<GraphEntry, GraphError> {
        match graph.name {
            Some(name) => {
                let graph_key = self.get_graph_key(name.as_str(), data_key)?;

                let mut data_errors = vec![];
                let mut graph_parents = vec![];
                if let Some(parents) = &graph.parents {
                    for p in parents {
                        match PublicKey::from_hex(p) {
                            Ok(public_key) => graph_parents.push(public_key),
                            Err(_) => data_errors.push(format!("parent is not a public key: {}", p))
                        }
                    }
                }

                let mut graph_descendants = vec![];
                if let Some(descendants) = &graph.descendants {
                    for d in descendants {
                        match PublicKey::from_hex(d.key.as_str()) {
                            Ok(key) => {
                                match GraphContent::from_hex(d.content.clone()) {
                                    Ok(content) => graph_descendants.push((key, content)),
                                    Err(_) => data_errors.push(format!("content is not a valid hex: {}", d.content))
                                }
                            }
                            Err(_) => data_errors.push(format!("key is not a public key: {}", d.key))
                        }
                    }
                }

                if data_errors.is_empty() {
                    let graph_content = GraphContent::from_hex(graph.content.clone())?;
                    let graph_entry = autonomi::graph::GraphEntry::new(&graph_key, graph_parents, graph_content, graph_descendants);
                    info!("Create graph entry from name [{}] for content [{}]", name, graph.content.clone());
                    let graph_entry_address: GraphEntryAddress = self.graph_entry_caching_client
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

    pub async fn get_graph_entry(&self, address: String, data_key: DataKey) -> Result<GraphEntry, GraphError> {
        let resolved_address = self.resolve_address(address, data_key).await?;
        let graph_entry_address = GraphEntryAddress::from_hex(resolved_address.as_str())?;
        match self.graph_entry_caching_client.graph_entry_get(&graph_entry_address).await {
            Ok(graph_entry) => {
                info!("Retrieved graph entry at address [{}] value [{}]", resolved_address, hex::encode(graph_entry.content.clone()));

                let mut graph_parents_vec = vec![];
                for p in graph_entry.parents.iter() {
                    graph_parents_vec.push(p.to_hex());
                }
                let graph_parents = if !graph_parents_vec.is_empty() { Some(graph_parents_vec) } else { None };

                let mut graph_descendants_vec = vec![];
                for (p, c) in graph_entry.descendants.iter() {
                    graph_descendants_vec.push(GraphDescendants::new(p.to_hex(), hex::encode(c)))
                }
                let graph_descendants = if !graph_descendants_vec.is_empty() { Some(graph_descendants_vec) } else { None };

                Ok(GraphEntry::new(None, hex::encode(graph_entry.content.clone()), Some(resolved_address), graph_parents, graph_descendants))
            }
            Err(e) => {
                warn!("Failed to retrieve graph entry at address [{}]: [{:?}]", resolved_address, e);
                Err(e)
            }
        }
    }

    pub async fn resolve_address(&self, address: String, data_key: DataKey) -> Result<String, GraphError> {
        Ok(if self.resolver_service.is_immutable_address(&address) {
            self.resolver_service.resolve_name(&address).await.unwrap_or(address)
        } else {
            let secret_key = self.get_secret_key(data_key)?;
            Client::register_key_from_name(&secret_key, address.as_str()).public_key().to_hex()
        })
    }

    fn get_graph_key(&self, name: &str, data_key: DataKey) -> Result<autonomi::SecretKey, CreateError> {
        let secret_key = self.get_secret_key(data_key)?;
        Ok(Client::register_key_from_name(&secret_key, name))
    }

    fn get_secret_key(&self, data_key: DataKey) -> Result<autonomi::SecretKey, CreateError> {
        match data_key {
            DataKey::Resolver => self.ant_tp_config.get_resolver_private_key(),
            DataKey::Personal => self.ant_tp_config.get_app_private_key(),
            DataKey::Custom(key) => match autonomi::SecretKey::from_hex(&key.as_str()) {
                Ok(secret_key) => Ok(secret_key),
                Err(e) => Err(CreateError::DataKeyMissing(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use crate::client::graph_entry_caching_client::MockGraphEntryCachingClient;
    use crate::service::resolver_service::MockResolverService;

    fn create_test_service(mock_client: MockGraphEntryCachingClient, mock_resolver: MockResolverService) -> GraphService {
        use clap::Parser;
        let ant_tp_config = AntTpConfig::parse_from(&[
            "anttp",
            "--app-private-key",
            "0000000000000000000000000000000000000000000000000000000000000001"
        ]);

        GraphService::new(mock_client, ant_tp_config, mock_resolver)
    }

    #[tokio::test]
    async fn test_create_graph_entry_success() {
        let mut mock_client = MockGraphEntryCachingClient::default();
        let mock_resolver = MockResolverService::default();
        let evm_wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);

        let name = "test_graph".to_string();
        let content = hex::encode([1u8; 32]);
        let graph_entry = GraphEntry::new(Some(name.clone()), content.clone(), None, None, None);

        let app_secret_key = autonomi::SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let graph_key = Client::register_key_from_name(&app_secret_key, name.as_str());
        let expected_address = GraphEntryAddress::new(graph_key.public_key());

        mock_client
            .expect_graph_entry_put()
            .with(always(), always(), eq(StoreType::Network))
            .times(1)
            .returning(move |_, _, _| Ok(expected_address));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.create_graph_entry(graph_entry, evm_wallet, StoreType::Network, DataKey::Personal).await;

        if let Err(ref e) = result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().address.unwrap(), expected_address.to_hex());
    }

    #[tokio::test]
    async fn test_get_graph_entry_success() {
        let mut mock_client = MockGraphEntryCachingClient::default();
        let mut mock_resolver = MockResolverService::default();

        let app_secret_key = autonomi::SecretKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let graph_address = GraphEntryAddress::new(app_secret_key.public_key());
        let address_hex = graph_address.to_hex();

        mock_resolver
            .expect_is_immutable_address()
            .with(eq(address_hex.clone()))
            .times(1)
            .returning(|_| true);

        mock_resolver
            .expect_resolve_name()
            .with(eq(address_hex.clone()))
            .times(1)
            .returning(move |addr| Some(addr.to_string()));

        let content = [1u8; 32];
        let graph_entry_data = autonomi::graph::GraphEntry::new(&app_secret_key, vec![], autonomi::graph::GraphContent::from(content), vec![]);

        mock_client
            .expect_graph_entry_get()
            .with(eq(graph_address))
            .times(1)
            .returning(move |_| Ok(graph_entry_data.clone()));

        let service = create_test_service(mock_client, mock_resolver);
        let result = service.get_graph_entry(address_hex, DataKey::Personal).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, hex::encode(content));
    }
}
