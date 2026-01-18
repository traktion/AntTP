use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::scratchpad_service::{Scratchpad as ServiceScratchpad, ScratchpadService};
use crate::controller::StoreType;
use crate::error::scratchpad_error::ScratchpadError;

pub mod public_scratchpad_proto {
    tonic::include_proto!("public_scratchpad");
}

use public_scratchpad_proto::public_scratchpad_service_server::PublicScratchpadService as PublicScratchpadServiceTrait;
pub use public_scratchpad_proto::public_scratchpad_service_server::PublicScratchpadServiceServer;
use public_scratchpad_proto::{PublicScratchpad, PublicScratchpadResponse, CreatePublicScratchpadRequest, UpdatePublicScratchpadRequest, GetPublicScratchpadRequest};

pub struct PublicScratchpadHandler {
    scratchpad_service: Data<ScratchpadService>,
    evm_wallet: Data<EvmWallet>,
}

impl PublicScratchpadHandler {
    pub fn new(scratchpad_service: Data<ScratchpadService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { scratchpad_service, evm_wallet }
    }
}

impl From<PublicScratchpad> for ServiceScratchpad {
    fn from(p: PublicScratchpad) -> Self {
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

impl From<ServiceScratchpad> for PublicScratchpad {
    fn from(p: ServiceScratchpad) -> Self {
        PublicScratchpad {
            name: p.name(),
            address: p.address(),
            data_encoding: p.data_encoding(),
            signature: p.signature(),
            content: p.content(),
            counter: p.counter(),
        }
    }
}

impl From<ScratchpadError> for Status {
    fn from(error: ScratchpadError) -> Self {
        Status::internal(error.to_string())
    }
}

#[tonic::async_trait]
impl PublicScratchpadServiceTrait for PublicScratchpadHandler {
    async fn create_public_scratchpad(
        &self,
        request: Request<CreatePublicScratchpadRequest>,
    ) -> Result<Response<PublicScratchpadResponse>, Status> {
        let req = request.into_inner();
        let scratchpad = req.scratchpad.ok_or_else(|| Status::invalid_argument("Scratchpad is required"))?;

        let result = self.scratchpad_service.create_scratchpad(
            req.name,
            ServiceScratchpad::from(scratchpad),
            self.evm_wallet.get_ref().clone(),
            false,
            StoreType::from(req.cache_only.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PublicScratchpadResponse {
            scratchpad: Some(PublicScratchpad::from(result)),
        }))
    }

    async fn update_public_scratchpad(
        &self,
        request: Request<UpdatePublicScratchpadRequest>,
    ) -> Result<Response<PublicScratchpadResponse>, Status> {
        let req = request.into_inner();
        let scratchpad = req.scratchpad.ok_or_else(|| Status::invalid_argument("Scratchpad is required"))?;

        let result = self.scratchpad_service.update_scratchpad(
            req.address,
            req.name,
            ServiceScratchpad::from(scratchpad),
            self.evm_wallet.get_ref().clone(),
            false,
            StoreType::from(req.cache_only.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PublicScratchpadResponse {
            scratchpad: Some(PublicScratchpad::from(result)),
        }))
    }

    async fn get_public_scratchpad(
        &self,
        request: Request<GetPublicScratchpadRequest>,
    ) -> Result<Response<PublicScratchpadResponse>, Status> {
        let req = request.into_inner();
        let result = self.scratchpad_service.get_scratchpad(req.address, None, false).await?;

        Ok(Response::new(PublicScratchpadResponse {
            scratchpad: Some(PublicScratchpad::from(result)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_from_proto_scratchpad() {
        let proto = PublicScratchpad {
            name: Some("test".to_string()),
            address: Some("0x123".to_string()),
            data_encoding: Some(1),
            signature: Some("sig".to_string()),
            content: Some("cont".to_string()),
            counter: Some(2),
        };
        let service: ServiceScratchpad = proto.clone().into();
        assert_eq!(service.name(), proto.name);
        assert_eq!(service.address(), proto.address);
        assert_eq!(service.data_encoding(), proto.data_encoding);
        assert_eq!(service.signature(), proto.signature);
        assert_eq!(service.content(), proto.content);
        assert_eq!(service.counter(), proto.counter);
    }

    #[tokio::test]
    async fn test_to_proto_scratchpad() {
        let service = ServiceScratchpad::new(
            Some("test".to_string()),
            Some("0x123".to_string()),
            Some(1),
            Some("sig".to_string()),
            Some("cont".to_string()),
            Some(2),
        );
        let proto: PublicScratchpad = service.into();
        assert_eq!(proto.name, Some("test".to_string()));
        assert_eq!(proto.address, Some("0x123".to_string()));
        assert_eq!(proto.data_encoding, Some(1));
        assert_eq!(proto.signature, Some("sig".to_string()));
        assert_eq!(proto.content, Some("cont".to_string()));
        assert_eq!(proto.counter, Some(2));
    }
}
