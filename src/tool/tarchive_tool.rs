#![allow(dead_code)]

use std::collections::HashMap;
use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use crate::controller::StoreType;
use crate::error::tarchive_error::TarchiveError;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateTarchiveRequest {
    #[schemars(description = "Base64 encoded content of the files to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateTarchiveRequest {
    #[schemars(description = "Address of the tarchive")]
    address: String,
    #[schemars(description = "Base64 encoded content of the files to add to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

impl From<TarchiveError> for ErrorData {
    fn from(error: TarchiveError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
    }
}

#[tool_router(router = tarchive_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new tarchive")]
    async fn create_tarchive(
        &self,
        Parameters(CreateTarchiveRequest { files, store_type }): Parameters<CreateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let public_archive_form = self.map_to_multipart_form(files)?;
        Ok(self.tarchive_service.create_tarchive(
            public_archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update an existing tarchive")]
    async fn update_tarchive(
        &self,
        Parameters(UpdateTarchiveRequest { address, files, store_type }): Parameters<UpdateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let public_archive_form = self.map_to_multipart_form(files)?;
        Ok(self.tarchive_service.update_tarchive(
            address,
            public_archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }
}
