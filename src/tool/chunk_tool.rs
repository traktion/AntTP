#![allow(dead_code)]

use actix_web::web::Data;
use ant_evm::EvmWallet;
use rmcp::{handler::server::{
    router::tool::ToolRouter,
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize};
use serde_json::json;
use crate::controller::StoreType;
use crate::error::chunk_error::ChunkError;
use crate::service::chunk_service::{Chunk, ChunkService};
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateChunkRequest {
    #[schemars(description = "Base64 encoded content of the chunk")]
    content: String,
    #[schemars(description = "Store chunk on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetChunkRequest {
    #[schemars(description = "Address of the chunk")]
    address: String,
}

impl From<Chunk> for CallToolResult {
    fn from(chunk: Chunk) -> CallToolResult {
        CallToolResult::structured(json!(chunk))
    }
}

impl From<ChunkError> for ErrorData {
    fn from(chunk_error: ChunkError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, chunk_error.to_string(), None)
    }
}


#[derive(Debug, Clone)]
pub struct ChunkTool {
    chunk_service: Data<ChunkService>,
    evm_wallet: Data<EvmWallet>,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = chunk_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new chunk with base64 encoded content")]
    async fn create_chunk(
        &self,
        Parameters(CreateChunkRequest { content, store_type }): Parameters<CreateChunkRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let chunk = Chunk::new(Some(content), None);
        Ok(self.chunk_service.create_chunk(
            chunk, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Get a chunk by its address")]
    async fn get_chunk(
        &self,
        Parameters(GetChunkRequest { address }): Parameters<GetChunkRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.chunk_service.get_chunk(address).await?.into())
    }
}
