#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::controller::StoreType;
use crate::error::scratchpad_error::ScratchpadError;
use crate::service::scratchpad_service::Scratchpad;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreatePublicScratchpadRequest {
    #[schemars(description = "Name of the public scratchpad")]
    name: String,
    #[schemars(description = "Base64 encoded content of the scratchpad")]
    content: String,
    #[schemars(description = "Store scratchpad on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdatePublicScratchpadRequest {
    #[schemars(description = "Address of the public scratchpad")]
    address: String,
    #[schemars(description = "Name of the public scratchpad")]
    name: String,
    #[schemars(description = "Base64 encoded content of the scratchpad")]
    content: String,
    #[schemars(description = "Store scratchpad on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPublicScratchpadRequest {
    #[schemars(description = "Address of the public scratchpad")]
    address: String,
}

impl From<Scratchpad> for CallToolResult {
    fn from(scratchpad: Scratchpad) -> CallToolResult {
        CallToolResult::structured(json!(scratchpad))
    }
}

impl From<ScratchpadError> for ErrorData {
    fn from(scratchpad_error: ScratchpadError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, scratchpad_error.to_string(), None)
    }
}

#[tool_router(router = public_scratchpad_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new public scratchpad")]
    async fn create_public_scratchpad(
        &self,
        Parameters(CreatePublicScratchpadRequest { name, content, store_type }): Parameters<CreatePublicScratchpadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let scratchpad = Scratchpad::new(Some(name), None, None, None, Some(content), None);
        Ok(self.scratchpad_service.create_scratchpad(
            scratchpad,
            self.evm_wallet.get_ref().clone(),
            false,
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update an existing public scratchpad")]
    async fn update_public_scratchpad(
        &self,
        Parameters(UpdatePublicScratchpadRequest { address, name, content, store_type }): Parameters<UpdatePublicScratchpadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let scratchpad = Scratchpad::new(None, None, None, None, Some(content), None);
        Ok(self.scratchpad_service.update_scratchpad(
            address,
            name,
            scratchpad,
            self.evm_wallet.get_ref().clone(),
            false,
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Get a public scratchpad by its address")]
    async fn get_public_scratchpad(
        &self,
        Parameters(GetPublicScratchpadRequest { address }): Parameters<GetPublicScratchpadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.scratchpad_service.get_scratchpad(address, None, false).await?.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_public_scratchpad_request_serialization() {
        let json = r#"{
            "name": "test_scratchpad",
            "content": "SGVsbG8gd29ybGQ=",
            "store_type": "memory"
        }"#;
        let request: CreatePublicScratchpadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test_scratchpad");
        assert_eq!(request.content, "SGVsbG8gd29ybGQ=");
        assert_eq!(request.store_type, "memory");
    }

    #[tokio::test]
    async fn test_update_public_scratchpad_request_serialization() {
        let json = r#"{
            "address": "0x123",
            "name": "test_scratchpad",
            "content": "SGVsbG8gd29ybGQ=",
            "store_type": "memory"
        }"#;
        let request: UpdatePublicScratchpadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.address, "0x123");
        assert_eq!(request.name, "test_scratchpad");
        assert_eq!(request.content, "SGVsbG8gd29ybGQ=");
        assert_eq!(request.store_type, "memory");
    }

    #[tokio::test]
    async fn test_get_public_scratchpad_request_serialization() {
        let json = r#"{
            "address": "0x123"
        }"#;
        let request: GetPublicScratchpadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.address, "0x123");
    }
}
