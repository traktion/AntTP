#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::model::resolve::Resolve;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct ResolveRequest {
    #[schemars(description = "Source name or address to resolve")]
    name: String,
}

impl From<Resolve> for CallToolResult {
    fn from(resolve: Resolve) -> CallToolResult {
        CallToolResult::structured(json!(resolve))
    }
}

#[tool_router(router = resolver_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Resolve a source name or address to its target address")]
    async fn resolve(
        &self,
        Parameters(ResolveRequest { name }): Parameters<ResolveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.resolver_service.resolve_name_item(&name).await {
            Some(resolve) => Ok(resolve.into()),
            None => Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Address for [{}] not found", name), None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_request_serialization() {
        let json = r#"{
            "name": "test_name"
        }"#;
        let request: ResolveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test_name");
    }
}
