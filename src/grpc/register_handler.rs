use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::register_service::{Register as ServiceRegister, RegisterService};
use crate::controller::StoreType;

pub mod register_proto {
    tonic::include_proto!("register");
}

use register_proto::register_service_server::RegisterService as RegisterServiceTrait;
pub use register_proto::register_service_server::RegisterServiceServer;
use register_proto::{Register, RegisterResponse, RegisterHistoryResponse, CreateRegisterRequest, UpdateRegisterRequest, GetRegisterRequest, GetRegisterHistoryRequest};
use crate::error::register_error::RegisterError;

pub struct RegisterHandler {
    register_service: Data<RegisterService>,
    evm_wallet: Data<EvmWallet>,
}

impl RegisterHandler {
    pub fn new(register_service: Data<RegisterService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { register_service, evm_wallet }
    }
}

impl From<Register> for ServiceRegister {
    fn from(r: Register) -> Self {
        ServiceRegister::new(r.name, r.content, r.address)
    }
}

impl From<ServiceRegister> for Register {
    fn from(r: ServiceRegister) -> Self {
        Register {
            name: r.name,
            content: r.content,
            address: r.address,
        }
    }
}

impl From<RegisterError> for Status {
    fn from(register_error: RegisterError) -> Self {
        Status::internal(register_error.to_string())
    }
}

#[tonic::async_trait]
impl RegisterServiceTrait for RegisterHandler {
    async fn create_register(
        &self,
        request: Request<CreateRegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let register = req.register.ok_or_else(|| Status::invalid_argument("Register is required"))?;

        let result = self.register_service.create_register(
            ServiceRegister::from(register),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.cache_only.unwrap_or_default()),
        ).await?;

        Ok(Response::new(RegisterResponse {
            register: Some(Register::from(result)),
        }))
    }

    async fn update_register(
        &self,
        request: Request<UpdateRegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let register = req.register.ok_or_else(|| Status::invalid_argument("Register is required"))?;

        let result = self.register_service.update_register(
            req.address,
            ServiceRegister::from(register),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.cache_only.unwrap_or_default()),
        ).await?;

        Ok(Response::new(RegisterResponse {
            register: Some(Register::from(result)),
        }))
    }

    async fn get_register(
        &self,
        request: Request<GetRegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let result = self.register_service.get_register(req.address).await?;

        Ok(Response::new(RegisterResponse {
            register: Some(Register::from(result)),
        }))
    }

    async fn get_register_history(
        &self,
        request: Request<GetRegisterHistoryRequest>,
    ) -> Result<Response<RegisterHistoryResponse>, Status> {
        let req = request.into_inner();
        let result = self.register_service.get_register_history(req.address).await?;

        Ok(Response::new(RegisterHistoryResponse {
            registers: result.into_iter().map(Register::from).collect(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::register_service::Register;
    use crate::error::register_error::RegisterError;
    use crate::error::GetError;

    #[test]
    fn test_from_register_proto_to_service() {
        let proto_register = Register {
            name: Some("test".to_string()),
            content: "content".to_string(),
            address: Some("address".to_string()),
        };
        let service_register = ServiceRegister::from(proto_register.clone());
        assert_eq!(service_register.name, proto_register.name);
        assert_eq!(service_register.content, proto_register.content);
        assert_eq!(service_register.address, proto_register.address);
    }

    #[test]
    fn test_from_service_to_register_proto() {
        let service_register = ServiceRegister::new(
            Some("test".to_string()),
            "content".to_string(),
            Some("address".to_string()),
        );
        let proto_register = Register::from(service_register.clone());
        assert_eq!(proto_register.name, service_register.name);
        assert_eq!(proto_register.content, service_register.content);
        assert_eq!(proto_register.address, service_register.address);
    }

    #[test]
    fn test_status_from_register_error() {
        let error = RegisterError::GetError(GetError::RecordNotFound("not found".to_string()));
        let status: Status = error.into();
        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("not found"));
    }
}
