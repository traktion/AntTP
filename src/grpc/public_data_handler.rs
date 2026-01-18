use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::public_data_service::PublicDataService;
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;

pub mod public_data_proto {
    tonic::include_proto!("public_data");
}

use public_data_proto::public_service_server::PublicService as PublicServiceTrait;
pub use public_data_proto::public_service_server::PublicServiceServer;
use public_data_proto::{CreatePublicDataRequest, PublicDataResponse, GetPublicDataRequest, GetPublicDataResponse};

pub struct PublicDataHandler {
    public_data_service: Data<PublicDataService>,
    evm_wallet: Data<EvmWallet>,
}

impl PublicDataHandler {
    pub fn new(public_data_service: Data<PublicDataService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { public_data_service, evm_wallet }
    }
}

impl From<PublicDataError> for Status {
    fn from(error: PublicDataError) -> Self {
        Status::internal(error.to_string())
    }
}

#[tonic::async_trait]
impl PublicServiceTrait for PublicDataHandler {
    async fn create_public_data(
        &self,
        request: Request<CreatePublicDataRequest>,
    ) -> Result<Response<PublicDataResponse>, Status> {
        let req = request.into_inner();

        let result = self.public_data_service.create_public_data(
            req.data.into(),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.cache_only.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PublicDataResponse {
            address: result.address.unwrap_or_default(),
        }))
    }

    async fn get_public_data(
        &self,
        request: Request<GetPublicDataRequest>,
    ) -> Result<Response<GetPublicDataResponse>, Status> {
        let req = request.into_inner();
        let bytes = self.public_data_service.get_public_data_binary(req.address).await?;

        Ok(Response::new(GetPublicDataResponse {
            data: bytes.into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CreateError;

    #[test]
    fn test_error_conversion() {
        let error = PublicDataError::CreateError(CreateError::InvalidData("test".to_string()));
        let status: Status = error.into();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("create error"));
    }
}
