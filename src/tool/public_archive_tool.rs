#![allow(dead_code)]

use std::io::Write;
use std::collections::HashMap;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use rmcp::{handler::server::{
    wrapper::Parameters,
}, schemars, tool, tool_router, ErrorData};
use rmcp::model::{CallToolResult, ErrorCode};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use crate::controller::StoreType;
use crate::error::public_archive_error::PublicArchiveError;
use crate::service::public_archive_service::{ArchiveContent, PublicArchiveForm, Upload};
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema)]
struct GetPublicArchiveRequest {
    #[schemars(description = "Address of the public archive")]
    address: String,
    #[schemars(description = "Path within the archive")]
    path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreatePublicArchiveRequest {
    #[schemars(description = "Base64 encoded content of the files to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional map of filename to its relative target path in the archive")]
    target_paths: Option<HashMap<String, String>>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdatePublicArchiveRequest {
    #[schemars(description = "Address of the public archive")]
    address: String,
    #[schemars(description = "Base64 encoded content of the files to add to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional map of filename to its relative target path in the archive")]
    target_paths: Option<HashMap<String, String>>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetStatusPublicArchiveRequest {
    #[schemars(description = "Id of upload")]
    id: String,
}

impl From<Upload> for CallToolResult {
    fn from(upload: Upload) -> CallToolResult {
        CallToolResult::structured(json!(upload))
    }
}

impl From<ArchiveContent> for CallToolResult {
    fn from(content: ArchiveContent) -> CallToolResult {
        CallToolResult::structured(json!(content))
    }
}

impl From<PublicArchiveError> for ErrorData {
    fn from(error: PublicArchiveError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
    }
}

#[tool_router(router = public_archive_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Get content or list files in a public archive")]
    async fn get_public_archive(
        &self,
        Parameters(GetPublicArchiveRequest { address, path }): Parameters<GetPublicArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.public_archive_service.get_public_archive(
            address,
            path,
        ).await?.into())
    }

    #[tool(description = "Create a new public archive")]
    async fn create_public_archive(
        &self,
        Parameters(CreatePublicArchiveRequest { files, target_paths, store_type }): Parameters<CreatePublicArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let public_archive_form = self.map_to_multipart_form(files, target_paths)?;
        Ok(self.public_archive_service.create_public_archive(
            public_archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    #[tool(description = "Update an existing public archive")]
    async fn update_public_archive(
        &self,
        Parameters(UpdatePublicArchiveRequest { address, files, target_paths, store_type }): Parameters<UpdatePublicArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let public_archive_form = self.map_to_multipart_form(files, target_paths)?;
        Ok(self.public_archive_service.update_public_archive(
            address,
            public_archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type)
        ).await?.into())
    }

    pub(crate) fn map_to_multipart_form(&self, files: HashMap<String, String>, target_paths: Option<HashMap<String, String>>) -> Result<MultipartForm<PublicArchiveForm>, ErrorData> {
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
        Ok(MultipartForm(PublicArchiveForm { files: temp_files, target_path: target_paths_vec }))
    }
}
