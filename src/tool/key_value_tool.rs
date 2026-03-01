#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::error::public_data_error::PublicDataError;
use crate::model::key_value::KeyValue;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct McpCreateKeyValueRequest {
    #[schemars(description = "Bucket name")]
    bucket: String,
    #[schemars(description = "Object name")]
    object: String,
    #[schemars(description = "Base64 encoded content of the value")]
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

    #[tool(description = "Create a new key/value pair in a bucket with base64 encoded content")]
    async fn create_key_value(
        &self,
        Parameters(McpCreateKeyValueRequest { bucket, object, content }): Parameters<McpCreateKeyValueRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let key_value = KeyValue::new(bucket.clone(), object.clone(), content);
        
        self.key_value_service.create_key_value(
            key_value,
            self.evm_wallet.get_ref().clone(),
            crate::controller::StoreType::Network,
        ).await.map_err(to_error_data)?;

        Ok(CallToolResult::structured(json!({
            "bucket": bucket,
            "object": object,
            "status": "created"
        })))
    }

    #[tool(description = "Get a key/value pair from a bucket with base64 encoded content")]
    async fn get_key_value(
        &self,
        Parameters(McpGetKeyValueRequest { bucket, object }): Parameters<McpGetKeyValueRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let key_value = self.key_value_service.get_key_value(bucket, object).await.map_err(to_error_data)?;
        
        Ok(CallToolResult::structured(json!({
            "content": key_value.content
        })))
    }
}
