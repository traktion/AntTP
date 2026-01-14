#![allow(dead_code)]

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreatePublicDataRequest {
    #[schemars(description = "Base64 encoded content of the public data")]
    content: String,
    #[schemars(description = "Store public data on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPublicDataRequest {
    #[schemars(description = "Address of the public data")]
    address: String,
}

impl From<PublicDataError> for ErrorData {
    fn from(error: PublicDataError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
    }
}

#[tool_router(router = public_data_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new public data with base64 encoded content")]
    async fn create_public_data(
        &self,
        Parameters(CreatePublicDataRequest { content, store_type }): Parameters<CreatePublicDataRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let decoded_content = BASE64_STANDARD.decode(content).map_err(|e| ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Invalid base64 content: {}", e), None))?;
        let chunk = self.public_data_service.create_public_data(
            Bytes::from(decoded_content),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?;
        Ok(CallToolResult::structured(json!(chunk)))
    }

    #[tool(description = "Get public data by its address")]
    async fn get_public_data(
        &self,
        Parameters(GetPublicDataRequest { address }): Parameters<GetPublicDataRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let bytes = self.public_data_service.get_public_data_binary(address.clone()).await?;
        let content = BASE64_STANDARD.encode(bytes);
        Ok(CallToolResult::structured(json!({
            "content": content,
            "address": address
        })))
    }
}
