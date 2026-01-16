#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use crate::controller::StoreType;
use crate::service::scratchpad_service::Scratchpad;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreatePrivateScratchpadRequest {
    #[schemars(description = "Name of the private scratchpad")]
    name: String,
    #[schemars(description = "Base64 encoded content of the scratchpad")]
    content: String,
    #[schemars(description = "Store scratchpad on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdatePrivateScratchpadRequest {
    #[schemars(description = "Address of the private scratchpad")]
    address: String,
    #[schemars(description = "Name of the private scratchpad")]
    name: String,
    #[schemars(description = "Base64 encoded content of the scratchpad")]
    content: String,
    #[schemars(description = "Store scratchpad on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPrivateScratchpadRequest {
    #[schemars(description = "Address of the private scratchpad")]
    address: String,
    #[schemars(description = "Name of the private scratchpad")]
    name: String,
}

#[tool_router(router = private_scratchpad_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new private scratchpad")]
    async fn create_private_scratchpad(
        &self,
        Parameters(CreatePrivateScratchpadRequest { name, content, store_type }): Parameters<CreatePrivateScratchpadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let scratchpad = Scratchpad::new(None, None, None, None, Some(content), None);
        Ok(self.scratchpad_service.create_scratchpad(
            name,
            scratchpad,
            self.evm_wallet.get_ref().clone(),
            true,
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update an existing private scratchpad")]
    async fn update_private_scratchpad(
        &self,
        Parameters(UpdatePrivateScratchpadRequest { address, name, content, store_type }): Parameters<UpdatePrivateScratchpadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let scratchpad = Scratchpad::new(None, None, None, None, Some(content), None);
        Ok(self.scratchpad_service.update_scratchpad(
            address,
            name,
            scratchpad,
            self.evm_wallet.get_ref().clone(),
            true,
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Get a private scratchpad by its address and name")]
    async fn get_private_scratchpad(
        &self,
        Parameters(GetPrivateScratchpadRequest { address, name }): Parameters<GetPrivateScratchpadRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.scratchpad_service.get_scratchpad(address, Some(name), true).await?.into())
    }
}
