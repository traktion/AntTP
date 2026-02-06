#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::controller::StoreType;
use crate::error::tarchive_error::TarchiveError;
use crate::service::public_archive_service::TarchiveForm;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct CreateTarchiveRequest {
    #[schemars(description = "Base64 encoded content of the files to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional map of filename to its relative target path in the archive")]
    target_paths: Option<HashMap<String, String>>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct UpdateTarchiveRequest {
    #[schemars(description = "Address of the tarchive")]
    address: String,
    #[schemars(description = "Base64 encoded content of the files to add to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional map of filename to its relative target path in the archive")]
    target_paths: Option<HashMap<String, String>>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

impl From<TarchiveError> for ErrorData {
    fn from(error: TarchiveError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
    }
}

#[tool_router(router = tarchive_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new tarchive")]
    async fn create_tarchive(
        &self,
        Parameters(CreateTarchiveRequest { files, target_paths, store_type }): Parameters<CreateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let tarchive_form = self.map_to_tarchive_multipart_form(files, target_paths)?;
        Ok(self.tarchive_service.create_tarchive(
            tarchive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update an existing tarchive")]
    async fn update_tarchive(
        &self,
        Parameters(UpdateTarchiveRequest { address, files, target_paths, store_type }): Parameters<UpdateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let tarchive_form = self.map_to_tarchive_multipart_form(files, target_paths)?;
        Ok(self.tarchive_service.update_tarchive(
            address,
            tarchive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }
    fn map_to_tarchive_multipart_form(&self, files: HashMap<String, String>, target_paths: Option<HashMap<String, String>>) -> Result<MultipartForm<TarchiveForm>, ErrorData> {
        let mut temp_files = Vec::new();
        let mut target_paths_vec = Vec::new();
        let target_paths = target_paths.unwrap_or_default();

        for (name, content_base64) in files {
            let content = BASE64_STANDARD.decode(content_base64).map_err(|e| 
                ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Invalid base64 content for file {}: {}", name, e), None)
            )?;
            
            let mut temp_file = tempfile::NamedTempFile::new().map_err(|e|
                ErrorData::new(ErrorCode::INTERNAL_ERROR, format!("Failed to create temp file: {}", e), None)
            )?;
            temp_file.write_all(&content).map_err(|e|
                ErrorData::new(ErrorCode::INTERNAL_ERROR, format!("Failed to write to temp file: {}", e), None)
            )?;

            let target_path = target_paths.get(&name).cloned().unwrap_or_default();
            target_paths_vec.push(actix_multipart::form::text::Text(target_path));

            temp_files.push(TempFile {
                file: temp_file,
                file_name: Some(name),
                content_type: None,
                size: content.len(),
            });
        }
        Ok(MultipartForm(TarchiveForm { files: temp_files, target_path: target_paths_vec }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_tarchive_request_serialization() {
        let mut files = HashMap::new();
        files.insert("test.txt".to_string(), "SGVsbG8gd29ybGQ=".to_string());
        let request = CreateTarchiveRequest {
            files,
            target_paths: None,
            store_type: "memory".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.store_type, "memory");
        assert_eq!(deserialized.files.get("test.txt").unwrap(), "SGVsbG8gd29ybGQ=");
    }

    #[tokio::test]
    async fn test_update_tarchive_request_serialization() {
        let mut files = HashMap::new();
        files.insert("test2.txt".to_string(), "VXBkYXRlZA==".to_string());
        let request = UpdateTarchiveRequest {
            address: "0x123".to_string(),
            files,
            target_paths: Some(HashMap::new()),
            store_type: "disk".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: UpdateTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0x123");
        assert_eq!(deserialized.store_type, "disk");
        assert_eq!(deserialized.files.get("test2.txt").unwrap(), "VXBkYXRlZA==");
    }
}
