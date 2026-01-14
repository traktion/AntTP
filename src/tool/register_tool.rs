#![allow(dead_code)]

use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::controller::StoreType;
use crate::error::register_error::RegisterError;
use crate::service::register_service::Register;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateRegisterRequest {
    #[schemars(description = "Name of the register")]
    name: String,
    #[schemars(description = "Content of the register (hex encoded)")]
    content: String,
    #[schemars(description = "Store register on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateRegisterRequest {
    #[schemars(description = "Address of the register")]
    address: String,
    #[schemars(description = "Name of the register")]
    name: String,
    #[schemars(description = "Content target of the register")]
    content: String,
    #[schemars(description = "Store register on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetRegisterRequest {
    #[schemars(description = "Address of the register")]
    address: String,
}

impl From<Register> for CallToolResult {
    fn from(register: Register) -> CallToolResult {
        CallToolResult::structured(json!(register))
    }
}

impl From<RegisterError> for ErrorData {
    fn from(register_error: RegisterError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, register_error.to_string(), None)
    }
}

#[tool_router(router = register_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Register a new register")]
    async fn create_register(
        &self,
        Parameters(CreateRegisterRequest { name, content, store_type }): Parameters<CreateRegisterRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let register = Register::new(Some(name), content, None);
        Ok(self.register_service.create_register(
            register,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
        ).await?.into())
    }

    #[tool(description = "Update an existing register")]
    async fn update_register(
        &self,
        Parameters(UpdateRegisterRequest { address, name, content, store_type }): Parameters<UpdateRegisterRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let register = Register::new(Some(name), content, None);
        Ok(self.register_service.update_register(
            address,
            register,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
        ).await?.into())
    }

    #[tool(description = "Get a register by its address")]
    async fn get_register(
        &self,
        Parameters(GetRegisterRequest { address }): Parameters<GetRegisterRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.register_service.get_register(address).await?.into())
    }

    #[tool(description = "Get register history by its address")]
    async fn get_register_history(
        &self,
        Parameters(GetRegisterRequest { address }): Parameters<GetRegisterRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let history = self.register_service.get_register_history(address).await?;
        Ok(CallToolResult::structured(json!(history)))
    }
}
