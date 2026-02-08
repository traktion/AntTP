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
use pointer_proto::{Pointer, PointerResponse, CreatePointerRequest, UpdatePointerRequest, GetPointerRequest};
use crate::error::pointer_error::PointerError;

pub struct PointerHandler {
    pointer_service: Data<PointerService>,
    evm_wallet: Data<EvmWallet>,
}

impl PointerHandler {
    pub fn new(pointer_service: Data<PointerService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { pointer_service, evm_wallet }
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

impl From<PointerError> for Status {
    fn from(pointer_error: PointerError) -> Self {
        Status::internal(pointer_error.to_string())
    }
}

#[tonic::async_trait]
impl PointerServiceTrait for PointerHandler {
    async fn create_pointer(
        &self,
        request: Request<CreatePointerRequest>,
    ) -> Result<Response<PointerResponse>, Status> {
        let req = request.into_inner();
        let pointer = req.pointer.ok_or_else(|| Status::invalid_argument("Pointer is required"))?;

        let result = self.pointer_service.create_pointer(
            ServicePointer::from(pointer),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
            DataKey::from(req.data_key.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PointerResponse {
            pointer: Some(Pointer::from(result)),
        }))
    }

    async fn update_pointer(
        &self,
        request: Request<UpdatePointerRequest>,
    ) -> Result<Response<PointerResponse>, Status> {
        let req = request.into_inner();
        let pointer = req.pointer.ok_or_else(|| Status::invalid_argument("Pointer is required"))?;

        let result = self.pointer_service.update_pointer(
            req.address,
            ServicePointer::from(pointer),
            StoreType::from(req.store_type.unwrap_or_default()),
            DataKey::from(req.data_key.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PointerResponse {
            pointer: Some(Pointer::from(result)),
        }))
    }

    async fn get_pointer(
        &self,
        request: Request<GetPointerRequest>,
    ) -> Result<Response<PointerResponse>, Status> {
        let req = request.into_inner();
        let result = self.pointer_service.get_pointer(req.address, DataKey::from(req.data_key.unwrap_or_default())).await?;

        Ok(Response::new(PointerResponse {
            pointer: Some(Pointer::from(result)),
        }))
    }
}
