use actix_http::header;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::{ContentLength, ContentType};
use actix_web::web::{Data, Payload};
use ant_evm::EvmWallet;
use log::debug;
use crate::error::chunk_error::ChunkError;
use crate::controller::get_store_type;
use crate::error::CreateError;
use crate::service::chunk_service::{Chunk, ChunkService};

#[utoipa::path(
    post,
    path = "/anttp-0/chunk",
    request_body(
        content = Chunk
    ),
    responses(
        (status = CREATED, description = "Chunk found successfully", body = Chunk),
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_chunk(
    chunk_service: Data<ChunkService>,
    evm_wallet_data: Data<EvmWallet>,
    chunk: web::Json<Chunk>,
    request: HttpRequest
) -> Result<HttpResponse, ChunkError> {
    debug!("Creating new chunk");
    Ok(HttpResponse::Created().json(
        chunk_service.create_chunk(chunk.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?))
}

#[utoipa::path(
    post,
    path = "/anttp-0/binary/chunk",
    request_body(
        content = Chunk,
        content_type = "application/octet-stream"
    ),
    responses(
        (status = CREATED, description = "Chunk uploaded successfully", body = Chunk),
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_chunk_binary(
    chunk_service: Data<ChunkService>,
    evm_wallet_data: Data<EvmWallet>,
    payload: Payload,
    request: HttpRequest
) -> Result<HttpResponse, ChunkError> {
    debug!("Creating new chunk");
    match payload.to_bytes().await {
        Ok(bytes) => {
            Ok(HttpResponse::Created().json(
                chunk_service.create_chunk_binary(bytes, evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?))
        }
        Err(_) => {
            Err(ChunkError::CreateError(CreateError::InvalidData("Failed to retrieve bytes from payload".to_string())))
        }
    }
}

#[utoipa::path(
    get,
    path = "/anttp-0/chunk/{address}",
    responses(
        (status = OK, description = "Chunk found successfully", body = Chunk),
        (status = NOT_FOUND, description = "Chunk was not found")
    ),
    params(
        ("address" = String, Path, description = "Chunk address"),
    )
)]
pub async fn get_chunk(
    path: web::Path<String>,
    chunk_service: Data<ChunkService>,
) -> Result<HttpResponse, ChunkError> {
    let address = path.into_inner();
    debug!("Getting chunk at [{}]", address);
    Ok(HttpResponse::Ok().json(chunk_service.get_chunk(address).await?))
}

#[utoipa::path(
    get,
    path = "/anttp-0/binary/chunk/{address}",
    responses(
        (status = OK, description = "Chunk found successfully", content_type = "application/octet-stream"),
        (status = NOT_FOUND, description = "Chunk was not found")
    ),
    params(
        ("address" = String, Path, description = "Chunk address"),
    )
)]
pub async fn get_chunk_binary(
    path: web::Path<String>,
    chunk_service: Data<ChunkService>,
) -> Result<HttpResponse, ChunkError> {
    let address = path.into_inner();
    debug!("Getting chunk at [{}]", address);
    let chunk = chunk_service.get_chunk_binary(address).await?;
    Ok(HttpResponse::Ok()
        .insert_header(ContentType::octet_stream())
        .insert_header(ContentLength(chunk.size()))
        .insert_header((header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))))
        .body(chunk.value))
}
