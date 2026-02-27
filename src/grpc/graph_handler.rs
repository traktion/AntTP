use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::graph_service::{GraphEntry as ServiceGraphEntry, GraphDescendants as ServiceGraphDescendants, GraphService};
use crate::controller::{DataKey, StoreType};
use crate::error::graph_error::GraphError;

pub mod graph_proto {
    tonic::include_proto!("graph");
}

use graph_proto::graph_service_server::GraphService as GraphServiceTrait;
pub use graph_proto::graph_service_server::GraphServiceServer;
use graph_proto::{GraphEntry, GraphDescendants, GraphResponse, CreateGraphEntryRequest, GetGraphEntryRequest};

pub struct GraphHandler {
    graph_service: Data<GraphService>,
    evm_wallet: Data<EvmWallet>,
}

impl GraphHandler {
    pub fn new(graph_service: Data<GraphService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { graph_service, evm_wallet }
    }
}

impl From<GraphDescendants> for ServiceGraphDescendants {
    fn from(d: GraphDescendants) -> Self {
        ServiceGraphDescendants::new(d.key, d.content)
    }
}

impl From<ServiceGraphDescendants> for GraphDescendants {
    fn from(d: ServiceGraphDescendants) -> Self {
        GraphDescendants {
            key: d.key,
            content: d.content,
        }
    }
}

impl From<GraphEntry> for ServiceGraphEntry {
    fn from(g: GraphEntry) -> Self {
        ServiceGraphEntry::new(
            g.name,
            g.content,
            g.address,
            if g.parents.is_empty() { None } else { Some(g.parents) },
            if g.descendants.is_empty() { None } else { Some(g.descendants.into_iter().map(ServiceGraphDescendants::from).collect()) },
        )
    }
}

impl From<ServiceGraphEntry> for GraphEntry {
    fn from(g: ServiceGraphEntry) -> Self {
        GraphEntry {
            name: g.name,
            content: g.content,
            address: g.address,
            parents: g.parents.unwrap_or_default(),
            descendants: g.descendants.unwrap_or_default().into_iter().map(GraphDescendants::from).collect(),
        }
    }
}

impl From<GraphError> for Status {
    fn from(graph_error: GraphError) -> Self {
        Status::internal(graph_error.to_string())
    }
}

#[tonic::async_trait]
impl GraphServiceTrait for GraphHandler {
    async fn create_graph_entry(
        &self,
        request: Request<CreateGraphEntryRequest>,
    ) -> Result<Response<GraphResponse>, Status> {
        let req = request.into_inner();
        let graph_entry = req.graph_entry.ok_or_else(|| Status::invalid_argument("Graph entry is required"))?;

        let result = self.graph_service.create_graph_entry(
            ServiceGraphEntry::from(graph_entry),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
            DataKey::from(req.data_key.unwrap_or_default()),
        ).await?;

        Ok(Response::new(GraphResponse {
            graph_entry: Some(GraphEntry::from(result)),
        }))
    }

    async fn get_graph_entry(
        &self,
        request: Request<GetGraphEntryRequest>,
    ) -> Result<Response<GraphResponse>, Status> {
        let req = request.into_inner();
        let result = self.graph_service.get_graph_entry(req.address, DataKey::from(req.data_key.unwrap_or_default())).await?;

        Ok(Response::new(GraphResponse {
            graph_entry: Some(GraphEntry::from(result)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::graph_service::{GraphEntry as ServiceGraphEntry, GraphDescendants as ServiceGraphDescendants};
    use graph_proto::{GraphEntry, GraphDescendants};

    #[test]
    fn test_graph_descendants_mapping() {
        let proto = GraphDescendants {
            key: "key".to_string(),
            content: "content".to_string(),
        };
        let service = ServiceGraphDescendants::from(proto.clone());
        assert_eq!(service.key, "key");
        assert_eq!(service.content, "content");

        let proto_back = GraphDescendants::from(service);
        assert_eq!(proto_back.key, "key");
        assert_eq!(proto_back.content, "content");
    }

    #[test]
    fn test_graph_entry_mapping() {
        let proto = GraphEntry {
            name: Some("name".to_string()),
            content: "content".to_string(),
            address: Some("address".to_string()),
            parents: vec!["parent1".to_string()],
            descendants: vec![GraphDescendants {
                key: "key".to_string(),
                content: "content".to_string(),
            }],
        };

        let service = ServiceGraphEntry::from(proto.clone());
        assert_eq!(service.name, Some("name".to_string()));
        assert_eq!(service.content, "content".to_string());
        assert_eq!(service.address, Some("address".to_string()));
        assert_eq!(service.parents, Some(vec!["parent1".to_string()]));
        assert_eq!(service.descendants.as_ref().unwrap().len(), 1);

        let proto_back = GraphEntry::from(service);
        assert_eq!(proto_back.name, Some("name".to_string()));
        assert_eq!(proto_back.content, "content".to_string());
        assert_eq!(proto_back.address, Some("address".to_string()));
        assert_eq!(proto_back.parents, vec!["parent1".to_string()]);
        assert_eq!(proto_back.descendants.len(), 1);
    }

    #[test]
    fn test_graph_entry_mapping_empty() {
        let proto = GraphEntry {
            name: None,
            content: "content".to_string(),
            address: None,
            parents: vec![],
            descendants: vec![],
        };

        let service = ServiceGraphEntry::from(proto.clone());
        assert_eq!(service.name, None);
        assert_eq!(service.parents, None);
        assert_eq!(service.descendants, None);

        let proto_back = GraphEntry::from(service);
        assert_eq!(proto_back.parents, Vec::<String>::new());
        assert_eq!(proto_back.descendants, Vec::<GraphDescendants>::new());
    }
}
