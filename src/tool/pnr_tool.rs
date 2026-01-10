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
use crate::error::pointer_error::PointerError;
use crate::model::pnr::{PnrRecord, PnrZone};
use crate::service::pnr_service::PnrService;

#[derive(Debug, Deserialize, JsonSchema)]
struct PnrRequest {
    #[schemars(description = "Name of the PNR zone")]
    name: String,
    #[schemars(description = "Target address of the default PNR record")]
    address: String,
    #[schemars(description = "Time To Live (TTL) for the default PNR record (default: 60)")]
    ttl: u64,
    #[schemars(description = "Store PNR zone on memory, disk or network")]
    store_type: String,
}

impl Into<CallToolResult> for PnrZone {
    fn into(self) -> CallToolResult {
        let default_record = self.records.get(0).expect("failed to get default PNR record");
        CallToolResult::success(vec![Content::text(format!(
            "Created a PNR zone with the name '{}' for the address '{}' with a TTL of '{}', a resolver pointer of '{}', and a personal pointer of '{}'.",
            self.name,
            default_record.address,
            default_record.ttl,
            self.resolver_address.expect("missing resolver address"),
            self.personal_address.expect("missing personal address")
        ))])
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

    #[tool(description = "Register PNR zone with default PNR record")]
    async fn register(
        &self,
        Parameters(PnrRequest { name, address, ttl, store_type }): Parameters<PnrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let ttl_or_default = if ttl == 0 { 60 } else { ttl };
        let pnr_zone = PnrZone::new(
            name, vec![PnrRecord::new(Some("".to_string()), address.clone(), ttl_or_default)], None, None
        );
        Ok(self.pnr_service.create_pnr(pnr_zone, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)).await?.into())
    }
}

#[tool_handler]
impl ServerHandler for PnrTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Pointer Name Resolver (PNR) tool for managing names resolving to addresses".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

