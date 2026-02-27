use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::pnr_service::PnrService;
use crate::controller::StoreType;
use crate::model::pnr::{PnrZone as ServicePnrZone, PnrRecord as ServicePnrRecord, PnrRecordType as ServicePnrRecordType};

pub mod pnr_proto {
    tonic::include_proto!("pnr");
}

use pnr_proto::pnr_service_server::PnrService as PnrServiceTrait;
pub use pnr_proto::pnr_service_server::PnrServiceServer;
use pnr_proto::{PnrZone, PnrRecord, PnrRecordType, PnrResponse, CreatePnrRequest, UpdatePnrRequest, UpdatePnrRecordRequest, GetPnrRequest};

pub struct PnrHandler {
    pnr_service: Data<PnrService>,
    evm_wallet: Data<EvmWallet>,
}

impl PnrHandler {
    pub fn new(pnr_service: Data<PnrService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { pnr_service, evm_wallet }
    }
}

impl From<PnrRecordType> for ServicePnrRecordType {
    fn from(t: PnrRecordType) -> Self {
        match t {
            PnrRecordType::A => ServicePnrRecordType::A,
            PnrRecordType::X => ServicePnrRecordType::X,
        }
    }
}

impl From<ServicePnrRecordType> for PnrRecordType {
    fn from(t: ServicePnrRecordType) -> Self {
        match t {
            ServicePnrRecordType::A => PnrRecordType::A,
            ServicePnrRecordType::X => PnrRecordType::X,
        }
    }
}

impl From<PnrRecord> for ServicePnrRecord {
    fn from(r: PnrRecord) -> Self {
        ServicePnrRecord {
            address: r.address,
            record_type: ServicePnrRecordType::from(PnrRecordType::try_from(r.record_type).unwrap_or(PnrRecordType::A)),
            ttl: r.ttl,
        }
    }
}

impl From<ServicePnrRecord> for PnrRecord {
    fn from(r: ServicePnrRecord) -> Self {
        PnrRecord {
            address: r.address,
            record_type: PnrRecordType::from(r.record_type) as i32,
            ttl: r.ttl,
        }
    }
}

impl From<PnrZone> for ServicePnrZone {
    fn from(z: PnrZone) -> Self {
        ServicePnrZone {
            name: z.name,
            records: z.records.into_iter().map(|(k, v)| (k, ServicePnrRecord::from(v))).collect(),
            resolver_address: z.resolver_address,
            personal_address: z.personal_address,
        }
    }
}

impl From<ServicePnrZone> for PnrZone {
    fn from(z: ServicePnrZone) -> Self {
        PnrZone {
            name: z.name,
            records: z.records.into_iter().map(|(k, v)| (k, PnrRecord::from(v))).collect(),
            resolver_address: z.resolver_address,
            personal_address: z.personal_address,
        }
    }
}

// Status mapping is already defined in pointer_handler.rs
// impl From<PointerError> for Status { ... }

#[tonic::async_trait]
impl PnrServiceTrait for PnrHandler {
    async fn create_pnr(
        &self,
        request: Request<CreatePnrRequest>,
    ) -> Result<Response<PnrResponse>, Status> {
        let req = request.into_inner();
        let pnr_zone = req.pnr_zone.ok_or_else(|| Status::invalid_argument("PnrZone is required"))?;

        let result = self.pnr_service.create_pnr(
            ServicePnrZone::from(pnr_zone),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PnrResponse {
            pnr_zone: Some(PnrZone::from(result)),
        }))
    }

    async fn update_pnr(
        &self,
        request: Request<UpdatePnrRequest>,
    ) -> Result<Response<PnrResponse>, Status> {
        let req = request.into_inner();
        let pnr_zone = req.pnr_zone.ok_or_else(|| Status::invalid_argument("PnrZone is required"))?;

        let result = self.pnr_service.update_pnr(
            req.name,
            ServicePnrZone::from(pnr_zone),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PnrResponse {
            pnr_zone: Some(PnrZone::from(result)),
        }))
    }

    async fn get_pnr(
        &self,
        request: Request<GetPnrRequest>,
    ) -> Result<Response<PnrResponse>, Status> {
        let req = request.into_inner();
        let result = self.pnr_service.get_pnr(req.name).await?;

        Ok(Response::new(PnrResponse {
            pnr_zone: Some(PnrZone::from(result)),
        }))
    }

    async fn update_pnr_record(
        &self,
        request: Request<UpdatePnrRecordRequest>,
    ) -> Result<Response<PnrResponse>, Status> {
        let req = request.into_inner();
        let pnr_record = req.pnr_record.ok_or_else(|| Status::invalid_argument("PnrRecord is required"))?;

        let result = self.pnr_service.update_pnr_record(
            req.name,
            req.record,
            ServicePnrRecord::from(pnr_record),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(PnrResponse {
            pnr_zone: Some(PnrZone::from(result)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::model::pnr::{PnrZone as ServicePnrZone, PnrRecord as ServicePnrRecord, PnrRecordType as ServicePnrRecordType};

    #[tokio::test]
    async fn test_create_pnr() {
        let proto_record = PnrRecord {
            address: "address1".to_string(),
            record_type: PnrRecordType::A as i32,
            ttl: 3600,
        };
        
        let service_record = ServicePnrRecord::from(proto_record.clone());
        assert_eq!(service_record.address, proto_record.address);
        assert!(matches!(service_record.record_type, ServicePnrRecordType::A));
        assert_eq!(service_record.ttl, proto_record.ttl);
        
        let proto_zone = PnrZone {
            name: "example.com".to_string(),
            records: HashMap::from([("www".to_string(), proto_record)]),
            resolver_address: None,
            personal_address: None,
        };
        
        let service_zone = ServicePnrZone::from(proto_zone.clone());
        assert_eq!(service_zone.name, proto_zone.name);
        assert_eq!(service_zone.records.len(), 1);
        assert_eq!(service_zone.records.get("www").unwrap().address, "address1");
    }

    #[tokio::test]
    async fn test_update_pnr_request_mapping() {
        let req = UpdatePnrRequest {
            name: "example.com".to_string(),
            pnr_zone: Some(PnrZone {
                name: "example.com".to_string(),
                records: HashMap::new(),
                resolver_address: None,
                personal_address: None,
            }),
            store_type: Some("memory".to_string()),
        };
        assert_eq!(req.name, "example.com");
        assert_eq!(req.pnr_zone.unwrap().name, "example.com");
        assert_eq!(req.store_type.unwrap(), "memory");
    }

    #[tokio::test]
    async fn test_get_pnr_request_mapping() {
        let req = GetPnrRequest {
            name: "example.com".to_string(),
        };
        assert_eq!(req.name, "example.com");
    }

    #[tokio::test]
    async fn test_update_pnr_record_request_mapping() {
        let req = UpdatePnrRecordRequest {
            name: "example.com".to_string(),
            record: "www".to_string(),
            pnr_record: Some(PnrRecord {
                address: "address1".to_string(),
                record_type: PnrRecordType::A as i32,
                ttl: 3600,
            }),
            store_type: Some("memory".to_string()),
        };
        assert_eq!(req.name, "example.com");
        assert_eq!(req.record, "www");
        assert_eq!(req.pnr_record.unwrap().address, "address1");
        assert_eq!(req.store_type.unwrap(), "memory");
    }
}
