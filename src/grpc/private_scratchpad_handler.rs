use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::scratchpad_service::{Scratchpad as ServiceScratchpad, ScratchpadService};
use crate::controller::StoreType;

pub mod scratchpad_proto {
    tonic::include_proto!("scratchpad_proto");
}

pub mod private_scratchpad_proto {
    tonic::include_proto!("private_scratchpad");
}

use private_scratchpad_proto::private_scratchpad_service_server::PrivateScratchpadService as PrivateScratchpadServiceTrait;
pub use private_scratchpad_proto::private_scratchpad_service_server::PrivateScratchpadServiceServer;
use private_scratchpad_proto::{PrivateScratchpadResponse, CreatePrivateScratchpadRequest, UpdatePrivateScratchpadRequest, GetPrivateScratchpadRequest};
use scratchpad_proto::Scratchpad;

pub struct PrivateScratchpadHandler {
    scratchpad_service: Data<ScratchpadService>,
    evm_wallet: Data<EvmWallet>,
}

impl PrivateScratchpadHandler {
    pub fn new(scratchpad_service: Data<ScratchpadService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { scratchpad_service, evm_wallet }
    }
}

impl From<Scratchpad> for ServiceScratchpad {
    fn from(p: Scratchpad) -> Self {
        ServiceScratchpad::new(
            p.name,
            p.address,
            p.data_encoding,
            p.signature,
            p.content,
            p.counter,
        )
    }
}

impl From<ServiceScratchpad> for Scratchpad {
    fn from(p: ServiceScratchpad) -> Self {
        Scratchpad {
            name: p.name,
            address: p.address,
            data_encoding: p.data_encoding,
            signature: p.signature,
            content: p.content,
            counter: p.counter,
        }
    }
}

#[tonic::async_trait]
impl PrivateScratchpadServiceTrait for PrivateScratchpadHandler {
    async fn create_private_scratchpad(
        &self,
        request: Request<CreatePrivateScratchpadRequest>,
    ) -> Result<Response<PrivateScratchpadResponse>, Status> {
        let req = request.into_inner();
        let proto_scratchpad = req.scratchpad.ok_or_else(|| Status::invalid_argument("Scratchpad is required"))?;
        let mut scratchpad = ServiceScratchpad::from(proto_scratchpad);
        scratchpad.name = Some(req.name);

        let result = self.scratchpad_service.create_scratchpad(
            scratchpad,
            self.evm_wallet.get_ref().clone(),
            true,
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PrivateScratchpadResponse {
            scratchpad: Some(Scratchpad::from(result)),
        }))
    }

    async fn update_private_scratchpad(
        &self,
        request: Request<UpdatePrivateScratchpadRequest>,
    ) -> Result<Response<PrivateScratchpadResponse>, Status> {
        let req = request.into_inner();
        let scratchpad = req.scratchpad.ok_or_else(|| Status::invalid_argument("Scratchpad is required"))?;

        let result = self.scratchpad_service.update_scratchpad(
            req.address,
            req.name,
            ServiceScratchpad::from(scratchpad),
            self.evm_wallet.get_ref().clone(),
            true,
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PrivateScratchpadResponse {
            scratchpad: Some(Scratchpad::from(result)),
        }))
    }

    async fn get_private_scratchpad(
        &self,
        request: Request<GetPrivateScratchpadRequest>,
    ) -> Result<Response<PrivateScratchpadResponse>, Status> {
        let req = request.into_inner();
        let result = self.scratchpad_service.get_scratchpad(
            req.address,
            Some(req.name),
            true,
        ).await?;

        Ok(Response::new(PrivateScratchpadResponse {
            scratchpad: Some(Scratchpad::from(result)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_proto_to_service() {
        let proto = Scratchpad {
            name: Some("test".to_string()),
            address: Some("0x123".to_string()),
            data_encoding: Some(1),
            signature: Some("sig".to_string()),
            content: Some("content".to_string()),
            counter: Some(10),
        };
        let service = ServiceScratchpad::from(proto.clone());
        assert_eq!(service.name, proto.name);
        assert_eq!(service.address, proto.address);
        assert_eq!(service.data_encoding, proto.data_encoding);
        assert_eq!(service.signature, proto.signature);
        assert_eq!(service.content, proto.content);
        assert_eq!(service.counter, proto.counter);
    }

    #[test]
    fn test_from_service_to_proto() {
        let service = ServiceScratchpad::new(
            Some("test".to_string()),
            Some("0x123".to_string()),
            Some(1),
            Some("sig".to_string()),
            Some("content".to_string()),
            Some(10),
        );
        let proto = Scratchpad::from(service.clone());
        assert_eq!(proto.name, service.name);
        assert_eq!(proto.address, service.address);
        assert_eq!(proto.data_encoding, service.data_encoding);
        assert_eq!(proto.signature, service.signature);
        assert_eq!(proto.content, service.content);
        assert_eq!(proto.counter, service.counter);
    }
}
