#![allow(dead_code)]

use crate::service::command_service::CommandList;
use crate::tool::McpTool;
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::{tool, tool_router, ErrorData};
use serde_json::json;

impl From<CommandList> for CallToolResult {
    fn from(command_list: CommandList) -> CallToolResult {
        CallToolResult::structured(json!(command_list))
    }
}

#[tool_router(router = command_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Get list of commands queued or executed")]
    async fn get_commands(
        &self,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.command_service.get_commands().await
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None))?
            .into())
    }
}
