#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::CallToolResult;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use crate::service::crypto_service::Crypto as ServiceCrypto;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CryptoVerifyRequest {
    #[schemars(description = "Public key as hex string")]
    public_key: String,
    #[schemars(description = "Map of data hex to signature hex")]
    crypto_map: HashMap<String, String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CryptoSignRequest {
    #[schemars(description = "List of data hex strings to sign")]
    data: Vec<String>,
}

#[tool_router(router = crypto_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Verify signatures of data using a public key")]
    async fn verify_signatures(
        &self,
        Parameters(CryptoVerifyRequest { public_key, crypto_map }): Parameters<CryptoVerifyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut data_map = HashMap::new();
        for (data_hex, signature_hex) in crypto_map {
            data_map.insert(data_hex, ServiceCrypto {
                signature: Some(signature_hex),
                verified: None,
            });
        }

        let result = self.crypto_service.verify(public_key, data_map);
        Ok(CallToolResult::structured(json!(result)))
    }

    #[tool(description = "Sign data using the application's secret key")]
    async fn create_signatures(
        &self,
        Parameters(CryptoSignRequest { data }): Parameters<CryptoSignRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut data_map = HashMap::new();
        for data_hex in data {
            data_map.insert(data_hex, ServiceCrypto {
                signature: None,
                verified: None,
            });
        }

        let result = self.crypto_service.sign(data_map);
        Ok(CallToolResult::structured(json!(result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use crate::service::crypto_service::CryptoService;
    use actix_web::web::Data;
    use blsttc::SecretKey;
    use ant_evm::EvmWallet;
    use crate::client::caching_client::CachingClient;
    use crate::service::pointer_service::PointerService;
    use crate::service::archive_service::ArchiveService;
    use crate::service::register_service::RegisterService;
    use crate::service::chunk_service::ChunkService;
    use crate::service::graph_service::GraphService;
    use crate::service::command_service::CommandService;
    use crate::service::pnr_service::PnrService;
    use crate::service::public_data_service::PublicDataService;
    use crate::service::public_archive_service::PublicArchiveService;
    use crate::service::tarchive_service::TarchiveService;
    use crate::service::scratchpad_service::ScratchpadService;
    use crate::service::resolver_service::ResolverService;
    use crate::service::key_value_service::KeyValueService;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_verify_signatures_tool() {
        let secret_key = SecretKey::random();
        let public_key = hex::encode(secret_key.public_key().to_bytes());
        let data = b"test data";
        let data_hex = hex::encode(data);
        let signature = hex::encode(secret_key.sign(data).to_bytes());

        let ant_tp_config = crate::config::anttp_config::AntTpConfig::parse_from(&["anttp"]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));
        
        let result = crypto_service.verify(public_key, {
            let mut data_map = HashMap::new();
            data_map.insert(data_hex.clone(), ServiceCrypto {
                signature: Some(signature.clone()),
                verified: None,
            });
            data_map
        });

        assert!(result.get(&data_hex).unwrap().verified.unwrap());
    }

    #[tokio::test]
    async fn test_create_signatures_tool() {
        let secret_key = SecretKey::random();
        let app_private_key_hex = secret_key.to_hex();
        let data_hex = hex::encode(b"hello world");

        let ant_tp_config = crate::config::anttp_config::AntTpConfig::parse_from(&["anttp", "--app-private-key", &app_private_key_hex]);
        let crypto_service = Data::new(CryptoService::new(ant_tp_config));
        
        let result = crypto_service.sign({
            let mut data_map = HashMap::new();
            data_map.insert(data_hex.clone(), ServiceCrypto {
                signature: None,
                verified: None,
            });
            data_map
        });

        assert!(result.get(&data_hex).unwrap().verified.unwrap());
        assert!(result.get(&data_hex).unwrap().signature.is_some());
    }
}
