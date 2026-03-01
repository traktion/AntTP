#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::error::public_data_error::PublicDataError;
use crate::tool::McpTool;
use bytes::Bytes;

#[derive(Debug, Deserialize, JsonSchema)]
struct McpCreateKeyValueRequest {
    #[schemars(description = "Bucket name")]
    bucket: String,
    #[schemars(description = "Object name")]
    object: String,
    #[schemars(description = "Content to store (plain text)")]
    content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct McpGetKeyValueRequest {
    #[schemars(description = "Bucket name")]
    bucket: String,
    #[schemars(description = "Object name")]
    object: String,
}

fn to_error_data(error: PublicDataError) -> ErrorData {
    ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
}

#[tool_router(router = key_value_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new key/value pair in a bucket")]
    async fn create_key_value(
        &self,
        Parameters(McpCreateKeyValueRequest { bucket, object, content }): Parameters<McpCreateKeyValueRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.key_value_service.create_key_value_binary(
            bucket.clone(),
            object.clone(),
            Bytes::from(content),
            self.evm_wallet.get_ref().clone(),
            crate::controller::StoreType::Network,
        ).await.map_err(to_error_data)?;

        Ok(CallToolResult::structured(json!({
            "bucket": bucket,
            "object": object,
            "status": "created"
        })))
    }

    #[tool(description = "Get a key/value pair from a bucket")]
    async fn get_key_value(
        &self,
        Parameters(McpGetKeyValueRequest { bucket, object }): Parameters<McpGetKeyValueRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let content_bytes = self.key_value_service.get_key_value_binary(bucket, object).await.map_err(to_error_data)?;
        let content = String::from_utf8_lossy(&content_bytes).to_string();
        
        Ok(CallToolResult::structured(json!({
            "content": content
        })))
    }
}
