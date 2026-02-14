use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use std::io::Write;
use crate::service::archive_service::{ArchiveForm, ArchiveService, ArchiveType as ServiceArchiveType, ArchiveResponse as ServiceArchiveResponse, ArchiveRaw as ServiceArchiveRaw, Upload as ServiceUpload};
use crate::controller::StoreType;
use crate::error::archive_error::ArchiveError;

pub mod archive_proto {
    tonic::include_proto!("archive");
}

use archive_proto::archive_service_server::ArchiveService as ArchiveServiceTrait;
pub use archive_proto::archive_service_server::ArchiveServiceServer;
use archive_proto::{ArchiveType, CreateArchiveRequest, UpdateArchiveRequest, TruncateArchiveRequest, GetArchiveRequest, PushArchiveRequest, ArchiveResponse, Item, File as ProtoFile};

pub struct ArchiveHandler {
    archive_service: Data<ArchiveService>,
    evm_wallet: Data<EvmWallet>,
}

impl ArchiveHandler {
    pub fn new(archive_service: Data<ArchiveService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { archive_service, evm_wallet }
    }

    fn map_to_multipart_form(&self, files: Vec<ProtoFile>) -> Result<MultipartForm<ArchiveForm>, Status> {
        let mut temp_files = Vec::new();
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
        }
        Ok(MultipartForm(ArchiveForm { files: temp_files }))
    }

    fn map_archive_type(proto_type: i32) -> ServiceArchiveType {
        match ArchiveType::try_from(proto_type).unwrap_or(ArchiveType::Public) {
            ArchiveType::Public => ServiceArchiveType::Public,
            ArchiveType::Tarchive => ServiceArchiveType::Tarchive,
        }
    }
}

impl From<ServiceUpload> for ArchiveResponse {
    fn from(upload: ServiceUpload) -> Self {
        ArchiveResponse {
            address: upload.address,
            ..Default::default()
        }
    }
}

impl From<ServiceArchiveResponse> for ArchiveResponse {
    fn from(res: ServiceArchiveResponse) -> Self {
        ArchiveResponse {
            address: Some(res.address),
            items: vec![], // Items are for listing/getting
            content: None,
        }
    }
}

impl From<ServiceArchiveRaw> for ArchiveResponse {
    fn from(res: ServiceArchiveRaw) -> Self {
        let items = res.items.into_iter().map(|pd| Item {
            name: pd.display,
            modified: pd.modified,
            size: pd.size,
            r#type: format!("{:?}", pd.path_type),
        }).collect();

        ArchiveResponse {
            address: Some(res.address),
            items,
            content: Some(res.content.into()),
        }
    }
}

impl From<ArchiveError> for Status {
    fn from(error: ArchiveError) -> Self {
        Status::internal(error.to_string())
    }
}

#[tonic::async_trait]
impl ArchiveServiceTrait for ArchiveHandler {
    async fn create_archive(
        &self,
        request: Request<CreateArchiveRequest>,
    ) -> Result<Response<ArchiveResponse>, Status> {
        let req = request.into_inner();
        let archive_form = self.map_to_multipart_form(req.files)?;
        let archive_type = Self::map_archive_type(req.archive_type as i32);

        match archive_type {
            ServiceArchiveType::Public => {
                let result = self.archive_service.create_public_archive(
                    req.path,
                    archive_form,
                    self.evm_wallet.get_ref().clone(),
                    StoreType::from(req.store_type.unwrap_or_default())
                ).await?;
                Ok(Response::new(ArchiveResponse::from(result)))
            },
            ServiceArchiveType::Tarchive => {
                let result = self.archive_service.create_tarchive(
                    req.path,
                    archive_form,
                    self.evm_wallet.get_ref().clone(),
                    StoreType::from(req.store_type.unwrap_or_default())
                ).await?;
                Ok(Response::new(ArchiveResponse::from(result)))
            }
        }
    }

    async fn update_archive(
        &self,
        request: Request<UpdateArchiveRequest>,
    ) -> Result<Response<ArchiveResponse>, Status> {
        let req = request.into_inner();
        let archive_form = self.map_to_multipart_form(req.files)?;
        let archive_type = Self::map_archive_type(req.archive_type as i32);

        let result = self.archive_service.update_archive(
            req.address,
            req.path,
            archive_form,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
            archive_type
        ).await?;

        Ok(Response::new(ArchiveResponse::from(result)))
    }

    async fn truncate_archive(
        &self,
        request: Request<TruncateArchiveRequest>,
    ) -> Result<Response<ArchiveResponse>, Status> {
        let req = request.into_inner();
        let archive_type = Self::map_archive_type(req.archive_type as i32);

        let result = self.archive_service.truncate_archive(
            req.address,
            req.path,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
            archive_type
        ).await?;

        Ok(Response::new(ArchiveResponse::from(result)))
    }

    async fn get_archive(
        &self,
        request: Request<GetArchiveRequest>,
    ) -> Result<Response<ArchiveResponse>, Status> {
        let req = request.into_inner();
        let archive_type = Self::map_archive_type(req.archive_type as i32);

        let result = self.archive_service.get_archive_binary(req.address, req.path, archive_type).await?;

        Ok(Response::new(ArchiveResponse::from(result)))
    }

    async fn push_archive(
        &self,
        request: Request<PushArchiveRequest>,
    ) -> Result<Response<ArchiveResponse>, Status> {
        let req = request.into_inner();
        let archive_type = Self::map_archive_type(req.archive_type as i32);

        let result = self.archive_service.push_archive(
            req.address,
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_else(|| "network".to_string())),
            archive_type
        ).await?;

        Ok(Response::new(ArchiveResponse::from(result)))
    }
}
