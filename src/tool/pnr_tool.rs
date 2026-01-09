#![allow(dead_code)]

use actix_web::web::Data;
use ant_evm::EvmWallet;
use rmcp::{
    ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::controller::CacheType;
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

#[derive(Debug, Serialize, JsonSchema)]
struct PnrResult {
    #[schemars(description = "name of the PNR record")]
    name: String,
    #[schemars(description = "address of the PNR map")]
    address: String,
    #[schemars(description = "resolver pointer address of the PNR record")]
    resolver_address: String,
    #[schemars(description = "personal pointer address of the PNR record")]
    personal_address: String,
}

impl From<PnrZone> for PnrResult {
    fn from(pnr_zone: PnrZone) -> Self {
        Self {
            name: pnr_zone.name.clone(),
            address: pnr_zone.records.get(0).unwrap().clone().address,
            resolver_address: pnr_zone.resolver_address.unwrap_or("".to_string()),
            personal_address: pnr_zone.personal_address.unwrap_or("".to_string()),
        }
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
    ) -> Result<Json<PnrResult>, String> {
        let pnr_zone = PnrZone::new(
            name, vec![PnrRecord::new(Some("".to_string()), address.clone(), 60)], None, None
        );
        let maybe_cache_only = if cache_only.is_some() { Some(CacheType::Memory) } else { None };
        match self.pnr_service.create_pnr(pnr_zone, self.evm_wallet.get_ref().clone(), maybe_cache_only).await {
            Ok(pnr_zone) => Ok(Json(PnrResult::from(pnr_zone))),
            Err(e) => Err(e.to_string()),
        }
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

