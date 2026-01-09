#![allow(dead_code)]

use actix_web::web::Data;
use ant_evm::EvmWallet;
use bytes::Bytes;
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
use crate::controller::{CacheType, DataKey};
use crate::model::pnr::{PnrRecord, PnrZone};
use crate::service::pnr_service::PnrService;
use crate::service::pointer_service::{Pointer, PointerService};
use crate::service::public_data_service::PublicDataService;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AntNsRequest {
    #[schemars(description = "the name of the PNR record")]
    name: String,
    #[schemars(description = "the target address of the PNR record")]
    address: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AntNsResult {
    #[schemars(description = "the address of the PNR map")]
    address: String,
}

impl From<String> for AntNsResult {
    fn from(address: String) -> Self {
        Self { address }
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
        Parameters(AntNsRequest { name, address }): Parameters<AntNsRequest>,
    ) -> Json<AntNsResult> {
        let pnr_zone = PnrZone::new(
            name, vec![PnrRecord::new(Some("".to_string()), address.clone(), 60)]
        );
        match self.pnr_service.create_pnr(pnr_zone, self.evm_wallet.get_ref().clone(), Some(CacheType::Memory)).await {
            Ok(pnr_zone) => Json(AntNsResult::from(pnr_zone.records.get(0).unwrap().address.to_string())),
            Err(_) => Json(AntNsResult::from(address.to_string())),
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

