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
use serde_json::json;
use crate::controller::CacheType;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::{PnrRecord, PnrZone};
use crate::service::pnr_service::PnrService;

#[derive(Debug, Deserialize, JsonSchema)]
struct PnrRequest {
    #[schemars(description = "name of the PNR record")]
    name: String,
    #[schemars(description = "target address of the PNR record")]
    address: String,
    #[schemars(description = "whether to cache to memory or disk or blank")]
    cache_only: Option<String>,
}

impl Into<CallToolResult> for PnrZone {
    fn into(self) -> CallToolResult {
        CallToolResult::success(vec![Content::text(json!(self).to_string())])
    }
}

impl From<PointerError> for ErrorData {
    fn from(pointer_error: PointerError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, pointer_error.to_string(), None)
    }
}


#[derive(Debug, Clone)]
pub struct PnrTool {
    pnr_service: Data<PnrService>,
    evm_wallet: Data<EvmWallet>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl PnrTool {
    pub fn new(pnr_service: Data<PnrService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { pnr_service, evm_wallet, tool_router: Self::tool_router(), }
    }

    #[tool(description = "Register PNR record with default configuration")]
    async fn register(
        &self,
        Parameters(PnrRequest { name, address, cache_only }): Parameters<PnrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let pnr_zone = PnrZone::new(
            name, vec![PnrRecord::new(Some("".to_string()), address.clone(), 60)], None, None
        );
        let cache_type = CacheType::from(cache_only.unwrap_or("".to_string()));
        let maybe_cache_only = if cache_type != CacheType::Network { Some(cache_type) } else { None };
        Ok(self.pnr_service.create_pnr(pnr_zone, self.evm_wallet.get_ref().clone(), maybe_cache_only).await?.into())
    }
}

#[tool_handler]
impl ServerHandler for PnrTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("PNR service for registering names".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

