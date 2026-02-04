use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use std::io::Write;
use crate::service::public_archive_service::{PublicArchiveForm, PublicArchiveService, Upload};
use crate::controller::StoreType;
use crate::error::public_archive_error::PublicArchiveError;

pub mod public_archive_proto {
    tonic::include_proto!("public_archive");
}

use public_archive_proto::public_archive_service_server::PublicArchiveService as PublicArchiveServiceTrait;
pub use public_archive_proto::public_archive_service_server::PublicArchiveServiceServer;
use public_archive_proto::{CreatePublicArchiveRequest, UpdatePublicArchiveRequest, TruncatePublicArchiveRequest, PublicArchiveResponse, File as ProtoFile, GetPublicArchiveRequest, GetPublicArchiveResponse};

pub struct PublicArchiveHandler {
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet: Data<EvmWallet>,
}

impl PublicArchiveHandler {
    pub fn new(public_archive_service: Data<PublicArchiveService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { public_archive_service, evm_wallet }
    }

    fn map_to_multipart_form(&self, files: Vec<ProtoFile>) -> Result<MultipartForm<PublicArchiveForm>, Status> {
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
        Ok(MultipartForm(PublicArchiveForm { files: temp_files, target_path: target_paths }))
    }
}

impl From<Upload> for PublicArchiveResponse {
    fn from(upload: Upload) -> Self {
        PublicArchiveResponse {
            address: upload.address,
        }
    }
}

impl From<PublicArchiveError> for Status {
    fn from(error: PublicArchiveError) -> Self {
        Status::internal(error.to_string())
    }
}

#[tonic::async_trait]
impl PublicArchiveServiceTrait for PublicArchiveHandler {
    async fn create_public_archive(
        &self,
        request: Request<CreatePublicArchiveRequest>,
    ) -> Result<Response<PublicArchiveResponse>, Status> {
        let req = request.into_inner();
        let public_archive_form = self.map_to_multipart_form(req.files)?;
        
        let result = self.public_archive_service.create_public_archive(
            public_archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default())
        ).await?;

        Ok(Response::new(PublicArchiveResponse::from(result)))
    }

    async fn update_public_archive(
        &self,
        request: Request<UpdatePublicArchiveRequest>,
    ) -> Result<Response<PublicArchiveResponse>, Status> {
        let req = request.into_inner();
        let public_archive_form = self.map_to_multipart_form(req.files)?;
        
        let result = self.public_archive_service.update_public_archive(
            req.address,
            public_archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default())
        ).await?;

        Ok(Response::new(PublicArchiveResponse::from(result)))
    }

    async fn truncate_public_archive(
        &self,
        request: Request<TruncatePublicArchiveRequest>,
    ) -> Result<Response<PublicArchiveResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.public_archive_service.truncate_public_archive(
            req.address,
            req.path,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default())
        ).await?;

        Ok(Response::new(PublicArchiveResponse::from(result)))
    }

    async fn get_public_archive(
        &self,
        request: Request<GetPublicArchiveRequest>,
    ) -> Result<Response<GetPublicArchiveResponse>, Status> {
        let req = request.into_inner();
        let result = self.public_archive_service.get_public_archive_binary(req.address, Some(req.path)).await?;

        Ok(Response::new(GetPublicArchiveResponse {
            address: Some(result.address),
            items: result.items,
            content: Some(result.content.into()),
        }))
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
        let response = PublicArchiveResponse::from(upload);
        assert_eq!(response.address, Some("0x1234".to_string()));
    }

    #[test]
    fn test_get_public_archive_response_mapping() {
        let items = vec!["file1.txt".to_string(), "file2.txt".to_string()];
        let content = bytes::Bytes::from("hello world");
        let address = "0x1234".to_string();
        
        let response = GetPublicArchiveResponse {
            address: Some(address.clone()),
            items: items.clone(),
            content: Some(content.clone().into()),
        };
        
        assert_eq!(response.address, Some(address));
        assert_eq!(response.items, items);
        assert_eq!(response.content, Some(content.to_vec()));
    }
}
