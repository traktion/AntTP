#![allow(dead_code)]

use actix_web::web::Data;
use ant_evm::EvmWallet;
use rmcp::{handler::server::{
    router::tool::ToolRouter,
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::controller::StoreType;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::{PnrRecord, PnrRecordType, PnrZone};
use crate::service::pnr_service::PnrService;
use crate::tool::McpTool;

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

#[derive(Debug, Serialize, JsonSchema)]
struct PnrResponse {
    #[schemars(description = "Name of the PNR zone")]
    name: String,
    #[schemars(description = "Target address of the default PNR record")]
    address: String,
    #[schemars(description = "Time To Live (TTL) for the default PNR record (default: 60)")]
    ttl: u64,
    #[schemars(description = "Resolver pointer address")]
    resolver_address: String,
    #[schemars(description = "Personal pointer address")]
    personal_address: String,
}

impl From<PnrZone> for CallToolResult {
    fn from(pnr_zone: PnrZone) -> CallToolResult {
        CallToolResult::structured(json!(pnr_zone))
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

#[tool_router(router = pnr_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Register PNR zone with default PNR record")]
    async fn register(
        &self,
        Parameters(PnrRequest { name, address, ttl, store_type }): Parameters<PnrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let ttl_or_default = if ttl == 0 { 60 } else { ttl };
        let pnr_zone = PnrZone::new(
            name,
            vec![PnrRecord::new(Some("".to_string()), address.clone(), PnrRecordType::X, ttl_or_default)],
            None,
            None
        );
        Ok(self.pnr_service.create_pnr(
            pnr_zone, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)
        ).await?.into())
    }
}
