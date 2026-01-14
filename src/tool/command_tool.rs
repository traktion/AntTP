#![allow(dead_code)]

use actix_web::web::Data;
use rmcp::{handler::server::{
    router::tool::ToolRouter,
}, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use serde_json::json;
use crate::service::command_service::{CommandList, CommandService};
use crate::tool::McpTool;

impl From<CommandList> for CallToolResult {
    fn from(command_list: CommandList) -> CallToolResult {
        CallToolResult::structured(json!(command_list))
    }
}

#[derive(Debug, Clone)]
pub struct CommandTool {
    command_service: Data<CommandService>,
    tool_router: ToolRouter<Self>,
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
