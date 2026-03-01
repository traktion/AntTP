use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::key_value_service::KeyValueService;
use crate::controller::StoreType;
use bytes::Bytes;

pub mod key_value_proto {
    tonic::include_proto!("key_value");
}

use key_value_proto::key_value_service_server::KeyValueService as KeyValueServiceTrait;
pub use key_value_proto::key_value_service_server::KeyValueServiceServer;
use key_value_proto::{CreateKeyValueRequest, GetKeyValueRequest, KeyValueResponse, GetKeyValueResponse};
use crate::error::public_data_error::PublicDataError;

pub struct KeyValueHandler {
    key_value_service: Data<KeyValueService>,
    evm_wallet: Data<EvmWallet>,
}

impl KeyValueHandler {
    pub fn new(key_value_service: Data<KeyValueService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { key_value_service, evm_wallet }
    }
}

fn to_status(error: PublicDataError) -> Status {
    Status::internal(error.to_string())
}

#[tonic::async_trait]
impl KeyValueServiceTrait for KeyValueHandler {
    async fn create_key_value(
        &self,
        request: Request<CreateKeyValueRequest>,
    ) -> Result<Response<KeyValueResponse>, Status> {
        let req = request.into_inner();
        
        self.key_value_service.create_key_value_binary(
            req.bucket.clone(),
            req.object.clone(),
            Bytes::from(req.content),
            self.evm_wallet.get_ref().clone(),
            StoreType::Network,
        ).await.map_err(to_status)?;

        Ok(Response::new(KeyValueResponse {
            bucket: req.bucket,
            object: req.object,
        }))
    }

    async fn get_key_value(
        &self,
        request: Request<GetKeyValueRequest>,
    ) -> Result<Response<GetKeyValueResponse>, Status> {
        let req = request.into_inner();
        
        let content = self.key_value_service.get_key_value_binary(
            req.bucket.clone(),
            req.object.clone(),
        ).await.map_err(to_status)?;

        Ok(Response::new(GetKeyValueResponse {
            bucket: req.bucket,
            object: req.object,
            content: content.to_vec(),
        }))
    }
}
