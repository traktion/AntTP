#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::controller::{StoreType, DataKey};
use crate::error::pointer_error::PointerError;
use crate::service::pointer_service::Pointer;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreatePointerRequest {
    #[schemars(description = "Name of the pointer")]
    name: String,
    #[schemars(description = "Content target of the pointer")]
    content: String,
    #[schemars(description = "Store pointer on memory, disk or network")]
    store_type: String,
    #[schemars(description = "Data key type (personal, resolver, or custom hex key)")]
    data_key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdatePointerRequest {
    #[schemars(description = "Address of the pointer")]
    address: String,
    #[schemars(description = "Name of the pointer")]
    name: String,
    #[schemars(description = "Content target of the pointer")]
    content: String,
    #[schemars(description = "Store pointer on memory, disk or network")]
    store_type: String,
    #[schemars(description = "Data key type (personal, resolver, or custom hex key)")]
    data_key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPointerRequest {
    #[schemars(description = "Address of the pointer")]
    address: String,
}

impl From<Pointer> for CallToolResult {
    fn from(pointer: Pointer) -> CallToolResult {
        CallToolResult::structured(json!(pointer))
    }
}

impl From<PointerError> for ErrorData {
    fn from(pointer_error: PointerError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, pointer_error.to_string(), None)
    }
}

#[tool_router(router = pointer_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new pointer")]
    async fn create_pointer(
        &self,
        Parameters(CreatePointerRequest { name, content, store_type, data_key }): Parameters<CreatePointerRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let pointer = Pointer::new(Some(name), content, None, None, None);
        Ok(self.pointer_service.create_pointer(
            pointer,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
            DataKey::from(data_key)
        ).await?.into())
    }

    #[tool(description = "Update an existing pointer")]
    async fn update_pointer(
        &self,
        Parameters(UpdatePointerRequest { address, name, content, store_type, data_key }): Parameters<UpdatePointerRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let pointer = Pointer::new(Some(name), content, None, None, None);
        Ok(self.pointer_service.update_pointer(
            address,
            pointer,
            StoreType::from(store_type),
            DataKey::from(data_key)
        ).await?.into())
    }

    #[tool(description = "Get a pointer by its address")]
    async fn get_pointer(
        &self,
        Parameters(GetPointerRequest { address }): Parameters<GetPointerRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.pointer_service.get_pointer(address).await?.into())
    }
}
