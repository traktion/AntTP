use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::pointer_service::{Pointer as ServicePointer, PointerService};
use crate::controller::{StoreType, DataKey};

pub mod pointer_proto {
    tonic::include_proto!("pointer");
}

use pointer_proto::pointer_service_server::PointerService as PointerServiceTrait;
pub use pointer_proto::pointer_service_server::PointerServiceServer;
use pointer_proto::{Pointer, PointerResponse, PostPointerRequest, PutPointerRequest, GetPointerRequest};

pub struct PointerHandler {
    pointer_service: Data<PointerService>,
    evm_wallet: Data<EvmWallet>,
}

impl PointerHandler {
    pub fn new(pointer_service: Data<PointerService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self {
            pointer_service,
            evm_wallet,
        }
    }
}

#[tonic::async_trait]
impl PointerServiceTrait for PointerHandler {
    async fn post_pointer(
        &self,
        request: Request<PostPointerRequest>,
    ) -> Result<Response<PointerResponse>, Status> {
        let req = request.into_inner();
        let pointer = req.pointer.ok_or_else(|| Status::invalid_argument("Pointer is required"))?;
        let store_type = StoreType::from(req.cache_only.unwrap_or_default());
        let data_key = DataKey::from(req.data_key.unwrap_or_default());

        let result = self.pointer_service.create_pointer(
            ServicePointer::from(pointer),
            self.evm_wallet.get_ref().clone(),
            store_type,
            data_key,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PointerResponse {
            pointer: Some(Pointer::from(result)),
        }))
    }

    async fn put_pointer(
        &self,
        request: Request<PutPointerRequest>,
    ) -> Result<Response<PointerResponse>, Status> {
        let req = request.into_inner();
        let pointer = req.pointer.ok_or_else(|| Status::invalid_argument("Pointer is required"))?;
        let store_type = StoreType::from(req.cache_only.unwrap_or_default());
        let data_key = DataKey::from(req.data_key.unwrap_or_default());

        let result = self.pointer_service.update_pointer(
            req.address,
            ServicePointer::from(pointer),
            store_type,
            data_key,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PointerResponse {
            pointer: Some(Pointer::from(result)),
        }))
    }

    async fn get_pointer(
        &self,
        request: Request<GetPointerRequest>,
    ) -> Result<Response<PointerResponse>, Status> {
        let req = request.into_inner();
        let result = self.pointer_service.get_pointer(req.address).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PointerResponse {
            pointer: Some(Pointer::from(result)),
        }))
    }
}

impl From<Pointer> for ServicePointer {
    fn from(p: Pointer) -> Self {
        ServicePointer {
            name: p.name,
            content: p.content,
            address: p.address,
            counter: p.counter,
            cost: p.cost,
        }
    }
}

impl From<ServicePointer> for Pointer {
    fn from(p: ServicePointer) -> Self {
        Pointer {
            name: p.name,
            content: p.content,
            address: p.address,
            counter: p.counter,
            cost: p.cost,
        }
    }
}
