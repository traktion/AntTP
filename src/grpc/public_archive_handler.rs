use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::public_archive_service::{PublicArchiveService, Upload};
use crate::controller::StoreType;
use crate::error::public_archive_error::PublicArchiveError;
use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;
use autonomi::files::PublicArchive;
use autonomi::Wallet;

pub mod public_archive_proto {
    tonic::include_proto!("public_archive");
}

use public_archive_proto::public_archive_service_server::PublicArchiveService as PublicArchiveServiceTrait;
pub use public_archive_proto::public_archive_service_server::PublicArchiveServiceServer;
use public_archive_proto::{CreatePublicArchiveRequest, UpdatePublicArchiveRequest, PublicArchiveResponse};

pub struct PublicArchiveHandler {
    public_archive_service: Data<PublicArchiveService>,
    evm_wallet: Data<EvmWallet>,
}

impl PublicArchiveHandler {
    pub fn new(public_archive_service: Data<PublicArchiveService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { public_archive_service, evm_wallet }
    }

    fn create_tmp_dir() -> Result<PathBuf, Status> {
        let random_name = Uuid::new_v4();
        let tmp_dir = std::env::temp_dir().join(random_name.to_string());
        fs::create_dir(&tmp_dir).map_err(|e| Status::internal(format!("Failed to create temp dir: {}", e)))?;
        Ok(tmp_dir)
    }

    fn write_files_to_tmp_dir(tmp_dir: &PathBuf, files: Vec<public_archive_proto::File>) -> Result<(), Status> {
        for file in files {
            let file_path = tmp_dir.join(sanitize_filename::sanitize(file.name));
            let mut f = StdFile::create(file_path).map_err(|e| Status::internal(format!("Failed to create temp file: {}", e)))?;
            f.write_all(&file.content).map_err(|e| Status::internal(format!("Failed to write to temp file: {}", e)))?;
        }
        Ok(())
    }

    fn purge_tmp_dir(tmp_dir: PathBuf) {
        let _ = fs::remove_dir_all(tmp_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::grpc::public_archive_handler::public_archive_proto;

    #[tokio::test]
    async fn test_create_tmp_dir() {
        let tmp_dir = PublicArchiveHandler::create_tmp_dir().unwrap();
        assert!(tmp_dir.exists());
        fs::remove_dir_all(tmp_dir).unwrap();
    }

    #[tokio::test]
    async fn test_write_files_to_tmp_dir() {
        let tmp_dir = PublicArchiveHandler::create_tmp_dir().unwrap();
        let files = vec![
            public_archive_proto::File {
                name: "test.txt".to_string(),
                content: b"hello world".to_vec(),
            },
        ];
        PublicArchiveHandler::write_files_to_tmp_dir(&tmp_dir, files).unwrap();
        let file_path = tmp_dir.join("test.txt");
        assert!(file_path.exists());
        let content = fs::read(file_path).unwrap();
        assert_eq!(content, b"hello world");
        fs::remove_dir_all(tmp_dir).unwrap();
    }
}

impl From<PublicArchiveError> for Status {
    fn from(err: PublicArchiveError) -> Self {
        Status::internal(err.to_string())
    }
}

impl From<Upload> for PublicArchiveResponse {
    fn from(upload: Upload) -> Self {
        PublicArchiveResponse {
            address: upload.get_address().clone(),
        }
    }
}

#[tonic::async_trait]
impl PublicArchiveServiceTrait for PublicArchiveHandler {
    async fn create_public_archive(
        &self,
        request: Request<CreatePublicArchiveRequest>,
    ) -> Result<Response<PublicArchiveResponse>, Status> {
        let req = request.into_inner();
        let evm_wallet = self.evm_wallet.get_ref().clone();
        let store_type = StoreType::from(req.cache_only.unwrap_or_default());

        let tmp_dir = Self::create_tmp_dir()?;
        if let Err(e) = Self::write_files_to_tmp_dir(&tmp_dir, req.files) {
            Self::purge_tmp_dir(tmp_dir);
            return Err(e);
        }

        let mut public_archive = PublicArchive::new();
        
        let result = self.upload_common(tmp_dir, &mut public_archive, evm_wallet, store_type).await?;

        Ok(Response::new(PublicArchiveResponse::from(result)))
    }

    async fn update_public_archive(
        &self,
        request: Request<UpdatePublicArchiveRequest>,
    ) -> Result<Response<PublicArchiveResponse>, Status> {
        let req = request.into_inner();
        let evm_wallet = self.evm_wallet.get_ref().clone();
        let store_type = StoreType::from(req.cache_only.unwrap_or_default());

        let tmp_dir = Self::create_tmp_dir()?;
        if let Err(e) = Self::write_files_to_tmp_dir(&tmp_dir, req.files) {
            Self::purge_tmp_dir(tmp_dir);
            return Err(e);
        }

        let mut public_archive = self.public_archive_service.get_caching_client().archive_get_public(
            autonomi::files::archive_public::ArchiveAddress::from_hex(&req.address)
                .map_err(|e| Status::invalid_argument(format!("Invalid address: {}", e)))?
        ).await.map_err(|e| Status::internal(format!("Failed to get archive: {}", e)))?;

        let result = self.upload_common(tmp_dir, &mut public_archive, evm_wallet, store_type).await?;

        Ok(Response::new(PublicArchiveResponse::from(result)))
    }
}

impl PublicArchiveHandler {
    async fn upload_common(&self, tmp_dir: PathBuf, public_archive: &mut PublicArchive, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, Status> {
        let result = self.public_archive_service.update_public_archive_from_dir(public_archive, tmp_dir.clone(), evm_wallet, store_type).await
            .map_err(|e| {
                let _ = fs::remove_dir_all(&tmp_dir);
                Status::internal(e.to_string())
            })?;

        let _ = fs::remove_dir_all(&tmp_dir);
        Ok(result)
    }
}
