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
use crate::model::antns::{AntNs, RecordType};
use crate::model::antns_list::AntNsList;
use crate::service::pointer_service::{Pointer, PointerService};
use crate::service::public_data_service::PublicDataService;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AntNsRequest {
    #[schemars(description = "the name of the AntNS record")]
    name: String,
    #[schemars(description = "the target address of the AntNS record")]
    address: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AntNsResult {
    #[schemars(description = "the address of the AntNS map")]
    address: String,
}

impl From<String> for AntNsResult {
    fn from(address: String) -> Self {
        Self { address }
    }
}


#[derive(Debug, Clone)]
pub struct AntNsTool {
    public_data_service: Data<PublicDataService>,
    pointer_service: Data<PointerService>,
    evm_wallet: Data<EvmWallet>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AntNsTool {
    pub fn new(public_data_service: Data<PublicDataService>, pointer_service: Data<PointerService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { public_data_service, pointer_service, evm_wallet, tool_router: Self::tool_router(), }
    }

    #[tool(description = "Register AntNS name with default configuration")]
    async fn register(
        &self,
        Parameters(AntNsRequest { name, address }): Parameters<AntNsRequest>,
    ) -> Json<AntNsResult> {
        let ant_ns_list = AntNsList::new(vec![AntNs::new(
            "".to_string(), address.clone(), RecordType::A, 60
        )]);
        match self.public_data_service.create_public_data(
            Bytes::from(serde_json::to_vec(&ant_ns_list).unwrap()),
            self.evm_wallet.get_ref().clone(),
            Some(CacheType::Memory)
        ).await {
            Ok(chunk) => {
                let personal_pointer_request = Pointer::new(
                    Some(name.clone()), chunk.address.unwrap(), None, None, None,
                );
                match self.pointer_service.create_pointer(
                    personal_pointer_request,
                    self.evm_wallet.get_ref().clone(),
                    Some(CacheType::Memory),
                    DataKey::Personal).await
                {
                    Ok(personal_pointer_result) => {
                        let resolver_pointer_request = Pointer::new(
                            Some(name.clone()), personal_pointer_result.address.unwrap(), None, None, None,
                        );
                        match self.pointer_service.create_pointer(
                            resolver_pointer_request,
                            self.evm_wallet.get_ref().clone(),
                            Some(CacheType::Memory),
                            DataKey::Resolver).await
                        {
                            Ok(_) => {
                                Json(AntNsResult::from(format!("{}->{}", name, address)))
                            },
                            Err(e) => Json(AntNsResult::from(e.to_string())),
                        }
                    },
                    Err(e) => Json(AntNsResult::from(e.to_string())),
                }
            },
            Err(e) => Json(AntNsResult::from(e.to_string())),
        }
    }
}

#[tool_handler]
impl ServerHandler for AntNsTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("AntNS service for registering names".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

