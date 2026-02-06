use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use std::io::Write;
use crate::service::public_archive_service::{PublicArchiveForm, TarchiveForm, Upload};
use crate::service::tarchive_service::TarchiveService;
use crate::controller::StoreType;
use crate::error::tarchive_error::TarchiveError;

pub mod tarchive_proto {
    tonic::include_proto!("tarchive");
}

use tarchive_proto::tarchive_service_server::TarchiveService as TarchiveServiceTrait;
pub use tarchive_proto::tarchive_service_server::TarchiveServiceServer;
use tarchive_proto::{CreateTarchiveRequest, UpdateTarchiveRequest, TarchiveResponse, File as ProtoFile};

pub struct TarchiveHandler {
    tarchive_service: Data<TarchiveService>,
    evm_wallet: Data<EvmWallet>,
}

impl TarchiveHandler {
    pub fn new(tarchive_service: Data<TarchiveService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { tarchive_service, evm_wallet }
    }

    fn map_to_multipart_form(&self, files: Vec<ProtoFile>) -> Result<MultipartForm<TarchiveForm>, Status> {
        let mut temp_files = Vec::new();
        let mut target_paths = Vec::new();
        for file in files {
            let mut temp_file = tempfile::NamedTempFile::new().map_err(|e|
                Status::internal(format!("Failed to create temp file: {}", e))
            )?;
            temp_file.write_all(&file.content).map_err(|e|
                Status::internal(format!("Failed to write to temp file: {}", e))
            )?;

            temp_files.push(TempFile {
                file: temp_file,
                file_name: Some(file.name),
                content_type: None,
                size: file.content.len(),
            });
            target_paths.push(actix_multipart::form::text::Text(file.target_path.unwrap_or_default()));
        }
        Ok(MultipartForm(TarchiveForm { files: temp_files, target_path: target_paths }))
    }
}

impl From<Upload> for TarchiveResponse {
    fn from(upload: Upload) -> Self {
        TarchiveResponse {
            address: upload.address,
        }
    }
}

impl From<TarchiveError> for Status {
    fn from(error: TarchiveError) -> Self {
        Status::internal(error.to_string())
    }
}

#[tonic::async_trait]
impl TarchiveServiceTrait for TarchiveHandler {
    async fn create_tarchive(
        &self,
        request: Request<CreateTarchiveRequest>,
    ) -> Result<Response<TarchiveResponse>, Status> {
        let req = request.into_inner();
        let tarchive_form = self.map_to_multipart_form(req.files)?;
        
        let result = self.tarchive_service.create_tarchive(
            tarchive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default())
        ).await?;

        Ok(Response::new(TarchiveResponse::from(result)))
    }

    async fn update_tarchive(
        &self,
        request: Request<UpdateTarchiveRequest>,
    ) -> Result<Response<TarchiveResponse>, Status> {
        let req = request.into_inner();
        let tarchive_form = self.map_to_multipart_form(req.files)?;
        
        let result = self.tarchive_service.update_tarchive(
            req.address,
            tarchive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default())
        ).await?;

        Ok(Response::new(TarchiveResponse::from(result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mapping() {
        let upload = Upload {
            address: Some("0x1234".to_string()),
        };
        let response = TarchiveResponse::from(upload);
        assert_eq!(response.address, Some("0x1234".to_string()));
    }
}
