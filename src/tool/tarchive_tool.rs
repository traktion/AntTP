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
use crate::service::public_archive_service::PublicArchiveForm;
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct CreateTarchiveRequest {
    #[schemars(description = "Base64 encoded content of the files to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional shared target path (directory) for all files in the archive")]
    path: Option<String>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct UpdateTarchiveRequest {
    #[schemars(description = "Address of the tarchive")]
    address: String,
    #[schemars(description = "Base64 encoded content of the files to add to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional shared target path (directory) for all files in the archive")]
    path: Option<String>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct TruncateTarchiveRequest {
    #[schemars(description = "Hex-encoded data address of the tarchive to truncate")]
    address: String,
    #[schemars(description = "The path within the tarchive to truncate (all files under this path will be removed)")]
    path: String,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct ListTarchiveRequest {
    #[schemars(description = "Hex-encoded data address of the tarchive to list")]
    address: String,
    #[schemars(description = "Optional path within the tarchive to list (e.g. 'folder/')")]
    path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct GetTarchiveRequest {
    #[schemars(description = "Hex-encoded data address of the tarchive to retrieve")]
    address: String,
    #[schemars(description = "Path within the tarchive to a specific file to retrieve")]
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct PushTarchiveRequest {
    #[schemars(description = "Hex-encoded data address of the tarchive to push")]
    address: String,
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
        Parameters(CreateTarchiveRequest { files, path, store_type }): Parameters<CreateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let tarchive_form = self.map_to_tarchive_multipart_form(files)?;
        Ok(self.tarchive_service.create_tarchive(
            path,
            tarchive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update an existing tarchive")]
    async fn update_tarchive(
        &self,
        Parameters(UpdateTarchiveRequest { address, files, path, store_type }): Parameters<UpdateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let tarchive_form = self.map_to_tarchive_multipart_form(files)?;
        Ok(self.tarchive_service.update_tarchive(
            address,
            path,
            tarchive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Truncate an existing tarchive (delete file or directory)")]
    async fn truncate_tarchive(
        &self,
        Parameters(TruncateTarchiveRequest { address, path, store_type }): Parameters<TruncateTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.tarchive_service.truncate_tarchive(
            address,
            path,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "List files in a tarchive")]
    async fn list_tarchive(
        &self,
        Parameters(ListTarchiveRequest { address, path }): Parameters<ListTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.tarchive_service.get_tarchive(
            address,
            path
        ).await?.into())
    }

    #[tool(description = "Get content of a file from a tarchive")]
    async fn get_tarchive(
        &self,
        Parameters(GetTarchiveRequest { address, path }): Parameters<GetTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self.tarchive_service.get_tarchive_binary(
            address,
            Some(path)
        ).await?;
        
        Ok(CallToolResult::success(vec![rmcp::model::Content::text(format!(
            "File content for {} in {}:\n\n{}",
            result.address,
            result.items.first().map(|i| i.display.as_str()).unwrap_or("unknown"),
            String::from_utf8_lossy(&result.content)
        ))]))
    }

    #[tool(description = "Push a tarchive to the network")]
    async fn push_tarchive(
        &self,
        Parameters(PushTarchiveRequest { address, store_type }): Parameters<PushTarchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.public_data_service.push_public_data(
            address,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    fn map_to_tarchive_multipart_form(&self, files: HashMap<String, String>) -> Result<MultipartForm<PublicArchiveForm>, ErrorData> {
        let mut temp_files = Vec::new();

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

            temp_files.push(TempFile {
                file: temp_file,
                file_name: Some(name),
                content_type: None,
                size: content.len(),
            });
        }
        Ok(MultipartForm(PublicArchiveForm { files: temp_files }))
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
            path: None,
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
            path: Some("secret".to_string()),
            store_type: "disk".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: UpdateTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0x123");
        assert_eq!(deserialized.store_type, "disk");
        assert_eq!(deserialized.files.get("test2.txt").unwrap(), "VXBkYXRlZA==");
        assert_eq!(deserialized.path, Some("secret".to_string()));
    }

    #[tokio::test]
    async fn test_truncate_tarchive_request_serialization() {
        let request = TruncateTarchiveRequest {
            address: "0x123".to_string(),
            path: "folder/".to_string(),
            store_type: "memory".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: TruncateTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0x123");
        assert_eq!(deserialized.path, "folder/");
        assert_eq!(deserialized.store_type, "memory");
    }

    #[tokio::test]
    async fn test_list_tarchive_request_serialization() {
        let request = ListTarchiveRequest {
            address: "0x123".to_string(),
            path: Some("folder/".to_string()),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ListTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0x123");
        assert_eq!(deserialized.path, Some("folder/".to_string()));
    }

    #[tokio::test]
    async fn test_get_tarchive_request_serialization() {
        let request = GetTarchiveRequest {
            address: "0x123".to_string(),
            path: "file.txt".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: GetTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0x123");
        assert_eq!(deserialized.path, "file.txt");
    }

    #[tokio::test]
    async fn test_push_tarchive_request_serialization() {
        let request = PushTarchiveRequest {
            address: "0x123".to_string(),
            store_type: "network".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: PushTarchiveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0x123");
        assert_eq!(deserialized.store_type, "network");
    }
}
