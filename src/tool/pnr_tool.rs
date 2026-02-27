#![allow(dead_code)]

use std::collections::HashMap;
use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::controller::StoreType;
use crate::model::pnr::{PnrRecord, PnrRecordType, PnrZone};
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

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdatePnrRequest {
    #[schemars(description = "Existing name of the PNR zone to update")]
    name: String,
    #[schemars(description = "Target address of the default PNR record")]
    address: String,
    #[schemars(description = "Time To Live (TTL) for the default PNR record (default: 60)")]
    ttl: u64,
    #[schemars(description = "Store PNR zone on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdatePnrRecordRequest {
    #[schemars(description = "Existing name of the PNR zone to update")]
    name: String,
    #[schemars(description = "Name of the PNR record to add/update")]
    record: String,
    #[schemars(description = "Target address of the PNR record")]
    address: String,
    #[schemars(description = "Time To Live (TTL) for the PNR record (default: 60)")]
    ttl: u64,
    #[schemars(description = "Type of the PNR record (A or X) (default: X)")]
    record_type: Option<String>,
    #[schemars(description = "Store PNR zone on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPnrRequest {
    #[schemars(description = "Name of the PNR zone")]
    name: String,
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

#[tool_router(router = pnr_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create PNR zone with default PNR record")]
    async fn create_pnr_zone(
        &self,
        Parameters(PnrRequest { name, address, ttl, store_type }): Parameters<PnrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let ttl_or_default = if ttl == 0 { 60 } else { ttl };
        let pnr_zone = PnrZone::new(
            name,
            HashMap::from([("".to_string(), PnrRecord::new(address.clone(), PnrRecordType::X, ttl_or_default))]),
            None,
            None
        );
        Ok(self.pnr_service.create_pnr(
            pnr_zone, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update PNR zone with default PNR record")]
    async fn update_pnr_zone(
        &self,
        Parameters(UpdatePnrRequest { name, address, ttl, store_type }): Parameters<UpdatePnrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let ttl_or_default = if ttl == 0 { 60 } else { ttl };
        let pnr_zone = PnrZone::new(
            name.clone(),
            HashMap::from([("".to_string(), PnrRecord::new(address.clone(), PnrRecordType::X, ttl_or_default))]),
            None,
            None
        );
        Ok(self.pnr_service.update_pnr(
            name, pnr_zone, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update or add PNR record to an existing PNR zone")]
    async fn update_pnr_record(
        &self,
        Parameters(UpdatePnrRecordRequest { name, record, address, ttl, record_type, store_type }): Parameters<UpdatePnrRecordRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let ttl_or_default = if ttl == 0 { 60 } else { ttl };
        let record_type = match record_type.as_deref() {
            Some("A") => PnrRecordType::A,
            _ => PnrRecordType::X,
        };
        let pnr_record = PnrRecord::new(address, record_type, ttl_or_default);
        Ok(self.pnr_service.update_pnr_record(
            name, record, pnr_record, self.evm_wallet.get_ref().clone(), StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Get PNR zone by name")]
    async fn get_pnr_zone(
        &self,
        Parameters(GetPnrRequest { name }): Parameters<GetPnrRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.pnr_service.get_pnr(name).await?.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_pnr_request_serialization() {
        let json = r#"{
            "name": "test_pnr"
        }"#;
        let request: GetPnrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test_pnr");
    }

    #[tokio::test]
    async fn test_pnr_request_serialization() {
        let json = r#"{
            "name": "test_pnr",
            "address": "0x123",
            "ttl": 60,
            "store_type": "memory"
        }"#;
        let request: PnrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test_pnr");
        assert_eq!(request.address, "0x123");
        assert_eq!(request.ttl, 60);
        assert_eq!(request.store_type, "memory");
    }

    #[tokio::test]
    async fn test_update_pnr_record_request_serialization() {
        let json = r#"{
            "name": "test_pnr",
            "record": "www",
            "address": "0x123",
            "ttl": 60,
            "record_type": "A",
            "store_type": "memory"
        }"#;
        let request: UpdatePnrRecordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test_pnr");
        assert_eq!(request.record, "www");
        assert_eq!(request.address, "0x123");
        assert_eq!(request.ttl, 60);
        assert_eq!(request.record_type, Some("A".to_string()));
        assert_eq!(request.store_type, "memory");
    }

    #[tokio::test]
    async fn test_update_pnr_request_serialization() {
        let json = r#"{
            "name": "test_pnr",
            "address": "0x123",
            "ttl": 60,
            "store_type": "memory"
        }"#;
        let request: UpdatePnrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test_pnr");
        assert_eq!(request.address, "0x123");
        assert_eq!(request.ttl, 60);
        assert_eq!(request.store_type, "memory");
    }
}
