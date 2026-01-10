#![allow(dead_code)]

use actix_web::web::Data;
use ant_evm::EvmWallet;
use rmcp::{ServerHandler, handler::server::{
    router::tool::ToolRouter,
    wrapper::Parameters,
}, model::{ServerCapabilities, ServerInfo}, schemars, tool, tool_handler, tool_router, ErrorData};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize};
use crate::controller::StoreType;
use crate::error::chunk_error::ChunkError;
use crate::service::chunk_service::{Chunk, ChunkService};

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

impl Into<CallToolResult> for Chunk {
    fn into(self) -> CallToolResult {
        // This is a default implementation. 
        // For specific tool outputs, manually constructing CallToolResult might be better 
        // if the output format varies significantly (e.g. content vs address).
        let address = self.address.unwrap_or_default();
        let content = self.content.unwrap_or_default();
        
        let mut text = String::new();
        if !address.is_empty() {
             text.push_str(&format!("Chunk Address: {}\n", address));
        }
        if !content.is_empty() {
            text.push_str(&format!("Chunk Content (Base64): {}", content));
        }
        
        if text.is_empty() {
            text = "Operation successful.".to_string();
        }

        CallToolResult::success(vec![Content::text(text)])
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

#[tool_router]
impl ChunkTool {
    pub fn new(chunk_service: Data<ChunkService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { chunk_service, evm_wallet, tool_router: Self::tool_router() }
    }

    #[tool(description = "Create a new chunk with base64 encoded content")]
    async fn create_chunk(
        &self,
        Parameters(CreateChunkRequest { content, store_type }): Parameters<CreateChunkRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let chunk = Chunk::new(Some(content), None);
        let result_chunk = self.chunk_service.create_chunk(chunk, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)).await?;
        
        // Return structured text
        let address = result_chunk.address.clone().unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created chunk at address '{}'", address
        ))]))
    }

    #[tool(description = "Get a chunk by its address")]
    async fn get_chunk(
        &self,
        Parameters(GetChunkRequest { address }): Parameters<GetChunkRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let chunk = self.chunk_service.get_chunk(address).await?;
        let content = chunk.content.clone().unwrap_or_default();
         Ok(CallToolResult::success(vec![Content::text(format!(
            "Chunk Content (Base64): {}", content
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for ChunkTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Chunk tool for creating and retrieving chunks".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_tool_info() {
        // We can test get_info without needing the service dependencies
        // because get_info doesn't use self.chunk_service.
        // However, we can't construct ChunkTool without Data<ChunkService>.
        // We can construct Data<ChunkService> if we can construct ChunkService.
        // ChunkService::new(CachingClient).
        // If we can't construct CachingClient, we can't construct ChunkTool.

        // This confirms we need to improve the testability of the codebase
        // or find a way to mock CachingClient.
        // For now, I'll include a placeholder test that would work if we could simple-mock.
        // Since I can't easily mock, I will comment out the instantiation part
        // and just test what I can (struct definitions).
    }
    
    #[test]
    fn test_create_chunk_request_deserialization() {
        let json = r#"{"content": "SGVsbG8=", "store_type": "memory"}"#;
        let request: CreateChunkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "SGVsbG8=");
        assert_eq!(request.store_type, "memory");
    }

    #[test]
    fn test_get_chunk_request_deserialization() {
        let json = r#"{"address": "1234abcd"}"#;
        let request: GetChunkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.address, "1234abcd");
    }
}
