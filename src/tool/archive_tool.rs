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
use serde_json::json;
use crate::controller::StoreType;
use crate::error::archive_error::ArchiveError;
use crate::service::archive_service::{ArchiveForm, ArchiveResponse, ArchiveType, Upload, ArchiveRaw};
use crate::tool::McpTool;

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
pub struct ArchiveTypeParam(pub String);

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct CreateArchiveRequest {
    #[schemars(description = "Type of archive: public or tarchive")]
    archive_type: String,
    #[schemars(description = "Base64 encoded content of the files to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional shared target path (directory) for all files in the archive")]
    path: Option<String>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct UpdateArchiveRequest {
    #[schemars(description = "Type of archive: public or tarchive")]
    archive_type: String,
    #[schemars(description = "Address of the archive")]
    address: String,
    #[schemars(description = "Base64 encoded content of the files to add to archive (map of filename to base64 content)")]
    files: HashMap<String, String>,
    #[schemars(description = "Optional shared target path (directory) for all files in the archive")]
    path: Option<String>,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct TruncateArchiveRequest {
    #[schemars(description = "Type of archive: public or tarchive")]
    archive_type: String,
    #[schemars(description = "Hex-encoded data address of the archive to truncate")]
    address: String,
    #[schemars(description = "The path within the archive to truncate (all files under this path will be removed)")]
    path: String,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct GetArchiveRequest {
    #[schemars(description = "Type of archive: public or tarchive")]
    archive_type: String,
    #[schemars(description = "Hex-encoded data address of the archive to retrieve")]
    address: String,
    #[schemars(description = "Path within the archive to a specific file to retrieve")]
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct PushArchiveRequest {
    #[schemars(description = "Type of archive: public or tarchive")]
    archive_type: String,
    #[schemars(description = "Hex-encoded data address of the archive to push")]
    address: String,
    #[schemars(description = "Store archive on memory, disk or network")]
    store_type: String,
}

impl From<ArchiveResponse> for CallToolResult {
    fn from(res: ArchiveResponse) -> CallToolResult {
        CallToolResult::structured(json!(res))
    }
}

impl From<Upload> for CallToolResult {
    fn from(upload: Upload) -> CallToolResult {
        CallToolResult::structured(json!(upload))
    }
}

impl From<ArchiveError> for ErrorData {
    fn from(error: ArchiveError) -> Self {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
    }
}

impl From<ArchiveRaw> for CallToolResult {
    fn from(res: ArchiveRaw) -> CallToolResult {
        CallToolResult::structured(json!({
            "address": res.address,
            "items": res.items,
            "content": BASE64_STANDARD.encode(&res.content)
        }))
    }
}

#[tool_router(router = archive_tool_router, vis = "pub")]
impl McpTool {

    #[tool(description = "Create a new archive")]
    async fn create_archive(
        &self,
        Parameters(CreateArchiveRequest { archive_type, files, path, store_type }): Parameters<CreateArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let archive_form = self.map_to_archive_multipart_form(files)?;
        let atype = self.parse_archive_type(&archive_type)?;
        
        match atype {
            ArchiveType::Public => Ok(self.archive_service.create_public_archive(
                path,
                archive_form,
                self.evm_wallet.get_ref().clone(),
                StoreType::from(store_type)
            ).await?.into()),
            ArchiveType::Tarchive => Ok(self.archive_service.create_tarchive(
                path,
                archive_form,
                self.evm_wallet.get_ref().clone(),
                StoreType::from(store_type)
            ).await?.into()),
        }
    }

    #[tool(description = "Update an existing archive")]
    async fn update_archive(
        &self,
        Parameters(UpdateArchiveRequest { archive_type, address, files, path, store_type }): Parameters<UpdateArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let archive_form = self.map_to_archive_multipart_form(files)?;
        let atype = self.parse_archive_type(&archive_type)?;
        Ok(self.archive_service.update_archive(
            address,
            path,
            archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
            atype
        ).await?.into())
    }

    #[tool(description = "Truncate an archive (delete file or directory)")]
    async fn truncate_archive(
        &self,
        Parameters(TruncateArchiveRequest { archive_type, address, path, store_type }): Parameters<TruncateArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let atype = self.parse_archive_type(&archive_type)?;
        Ok(self.archive_service.truncate_archive(
            address,
            path,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
            atype
        ).await?.into())
    }

    #[tool(description = "Get a file from an archive")]
    async fn get_archive(
        &self,
        Parameters(GetArchiveRequest { archive_type, address, path }): Parameters<GetArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let atype = self.parse_archive_type(&archive_type)?;
        Ok(self.archive_service.get_archive_binary(address, Some(path), atype).await?.into())
    }

    #[tool(description = "Push a staged archive from cache to a target store type (default: network)")]
    async fn push_archive(
        &self,
        Parameters(PushArchiveRequest { archive_type, address, store_type }): Parameters<PushArchiveRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let atype = self.parse_archive_type(&archive_type)?;
        Ok(self.archive_service.push_archive(
            address,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(store_type),
            atype
        ).await?.into())
    }

    pub(crate) fn map_to_archive_multipart_form(&self, files: HashMap<String, String>) -> Result<MultipartForm<ArchiveForm>, ErrorData> {
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
        Ok(MultipartForm(ArchiveForm { files: temp_files }))
    }

    fn parse_archive_type(&self, archive_type: &str) -> Result<ArchiveType, ErrorData> {
        match archive_type.to_lowercase().as_str() {
            "public" => Ok(ArchiveType::Public),
            "tarchive" => Ok(ArchiveType::Tarchive),
            _ => Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Invalid archive type: {}. Must be 'public' or 'tarchive'", archive_type), None)),
        }
    }
}
