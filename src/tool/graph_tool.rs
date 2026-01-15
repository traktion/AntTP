#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::controller::StoreType;
use crate::error::graph_error::GraphError;
use crate::service::graph_service::{GraphEntry, GraphDescendants};
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateGraphEntryRequest {
    #[schemars(description = "Name of the graph entry")]
    name: String,
    #[schemars(description = "Content of the graph entry (hex encoded)")]
    content: String,
    #[schemars(description = "Addresses of parent graph entries")]
    parents: Option<Vec<String>>,
    #[schemars(description = "Descendants of the graph entry")]
    descendants: Option<Vec<GraphDescendantsRequest>>,
    #[schemars(description = "Store graph entry on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GraphDescendantsRequest {
    #[schemars(description = "Public key of the descendant")]
    public_key: String,
    #[schemars(description = "Content of the descendant (hex encoded)")]
    content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetGraphEntryRequest {
    #[schemars(description = "Address of the graph entry")]
    address: String,
}

impl From<GraphEntry> for CallToolResult {
    fn from(graph_entry: GraphEntry) -> CallToolResult {
        CallToolResult::structured(json!(graph_entry))
    }
}

impl From<GraphError> for ErrorData {
    fn from(graph_error: GraphError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, graph_error.to_string(), None)
    }
}

#[tool_router(router = graph_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new graph entry")]
    async fn create_graph_entry(
        &self,
        Parameters(CreateGraphEntryRequest { name, content, parents, descendants, store_type }): Parameters<CreateGraphEntryRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let descendants = descendants.map(|ds| {
            ds.into_iter()
                .map(|d| GraphDescendants::new(d.public_key, d.content))
                .collect()
        });
        let graph_entry = GraphEntry::new(Some(name), content, None, parents, descendants);
        Ok(self.graph_service.create_graph_entry(
            graph_entry,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
        ).await?.into())
    }

    #[tool(description = "Get a graph entry by its address")]
    async fn get_graph_entry(
        &self,
        Parameters(GetGraphEntryRequest { address }): Parameters<GetGraphEntryRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.graph_service.get_graph_entry(address).await?.into())
    }
}
