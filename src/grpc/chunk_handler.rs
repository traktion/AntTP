use tonic::{Request, Response, Status};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use crate::service::chunk_service::{Chunk as ServiceChunk, ChunkService};
use crate::controller::StoreType;
use bytes::Bytes;

pub mod chunk_proto {
    tonic::include_proto!("chunk");
}

use chunk_proto::chunk_service_server::ChunkService as ChunkServiceTrait;
pub use chunk_proto::chunk_service_server::ChunkServiceServer;
use chunk_proto::{Chunk, ChunkResponse, CreateChunkRequest, CreateChunkBinaryRequest, GetChunkRequest, GetChunkBinaryResponse};
use crate::error::chunk_error::ChunkError;

pub struct ChunkHandler {
    chunk_service: Data<ChunkService>,
    evm_wallet: Data<EvmWallet>,
}

impl ChunkHandler {
    pub fn new(chunk_service: Data<ChunkService>, evm_wallet: Data<EvmWallet>) -> Self {
        Self { chunk_service, evm_wallet }
    }
}

impl From<Chunk> for ServiceChunk {
    fn from(c: Chunk) -> Self {
        ServiceChunk {
            content: c.content,
            address: c.address,
        }
    }
}

impl From<ServiceChunk> for Chunk {
    fn from(c: ServiceChunk) -> Self {
        Chunk {
            content: c.content,
            address: c.address,
        }
    }
}

impl From<ChunkError> for Status {
    fn from(chunk_error: ChunkError) -> Self {
        Status::internal(chunk_error.to_string())
    }
}

#[tonic::async_trait]
impl ChunkServiceTrait for ChunkHandler {
    async fn create_chunk(
        &self,
        request: Request<CreateChunkRequest>,
    ) -> Result<Response<ChunkResponse>, Status> {
        let req = request.into_inner();
        let chunk = req.chunk.ok_or_else(|| Status::invalid_argument("Chunk is required"))?;

        let result = self.chunk_service.create_chunk(
            ServiceChunk::from(chunk),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(ChunkResponse {
            chunk: Some(Chunk::from(result)),
        }))
    }

    async fn create_chunk_binary(
        &self,
        request: Request<CreateChunkBinaryRequest>,
    ) -> Result<Response<ChunkResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.chunk_service.create_chunk_binary(
            Bytes::from(req.data),
            self.evm_wallet.get_ref().clone(),
            StoreType::from(req.store_type.unwrap_or_default()),
        ).await?;

        Ok(Response::new(ChunkResponse {
            chunk: Some(Chunk::from(result)),
        }))
    }

    async fn get_chunk(
        &self,
        request: Request<GetChunkRequest>,
    ) -> Result<Response<ChunkResponse>, Status> {
        let req = request.into_inner();
        let result = self.chunk_service.get_chunk(req.address).await?;

        Ok(Response::new(ChunkResponse {
            chunk: Some(Chunk::from(result)),
        }))
    }

    async fn get_chunk_binary(
        &self,
        request: Request<GetChunkRequest>,
    ) -> Result<Response<GetChunkBinaryResponse>, Status> {
        let req = request.into_inner();
        let result = self.chunk_service.get_chunk_binary(req.address).await?;

        Ok(Response::new(GetChunkBinaryResponse {
            data: result.value.to_vec(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_chunk() {
        let proto_chunk = Chunk {
            content: Some("test content".to_string()),
            address: Some("0x123".to_string()),
        };
        let service_chunk = ServiceChunk::from(proto_chunk.clone());
        assert_eq!(service_chunk.content, proto_chunk.content);
        assert_eq!(service_chunk.address, proto_chunk.address);
    }

    #[test]
    fn test_from_service_chunk() {
        let service_chunk = ServiceChunk {
            content: Some("test content".to_string()),
            address: Some("0x123".to_string()),
        };
        let proto_chunk = Chunk::from(service_chunk.clone());
        assert_eq!(proto_chunk.content, service_chunk.content);
        assert_eq!(proto_chunk.address, service_chunk.address);
    }
}
