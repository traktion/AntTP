use actix_web::{web, Responder};
use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Data, Payload};
use ant_evm::EvmWallet;
use autonomi::Client;
use foyer::HybridCache;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::ClientCacheState;
use crate::config::anttp_config::AntTpConfig;
use crate::service::chunk_service::{Chunk, ChunkService};

#[utoipa::path(
    post,
    path = "/anttp-0/chunk",
    request_body(
        content = Chunk
    ),
    responses(
        (status = 200, description = "Chunk found successfully", body = Chunk),
        (status = NOT_FOUND, description = "Chunk was not found")
    ),
)]
pub async fn post_chunk(
    autonomi_client_data: Data<Option<Client>>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    chunk: web::Json<Chunk>,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data),
    );

    info!("Creating new chunk");
    chunk_service.create_chunk(chunk.into_inner(), evm_wallet).await
}

#[utoipa::path(
    post,
    path = "/anttp-0/binary/chunk/{address}",
    request_body(
        content_type = "application/octet-stream"
    ),
    responses(
        (status = 200, description = "Chunk found successfully", body = Chunk),
        (status = NOT_FOUND, description = "Chunk was not found")
    ),
)]
pub async fn post_chunk_binary(
    autonomi_client_data: Data<Option<Client>>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    payload: Payload,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data),
    );

    info!("Creating new chunk");
    match payload.to_bytes().await {
        Ok(bytes) => {
            chunk_service.create_chunk_binary(bytes, evm_wallet).await
        }
        Err(_) => {
            Err(ErrorInternalServerError("Failed to retrieve bytes from payload"))
        }
    }
}

#[utoipa::path(
    get,
    path = "/anttp-0/chunk/{address}",
    responses(
        (status = 200, description = "Chunk found successfully", body = Chunk),
        (status = NOT_FOUND, description = "Chunk was not found")
    ),
    params(
        ("address" = String, Path, description = "Chunk address"),
    )
)]
pub async fn get_chunk(
    path: web::Path<String>,
    autonomi_client_data: Data<Option<Client>>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data)
    );

    info!("Getting chunk at [{}]", address);
    chunk_service.get_chunk(address).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/binary/chunk/{address}",
    responses(
        (status = 200, description = "Chunk found successfully", content_type = "application/octet-stream"),
        (status = NOT_FOUND, description = "Chunk was not found")
    ),
    params(
        ("address" = String, Path, description = "Chunk address"),
    )
)]
pub async fn get_chunk_binary(
    path: web::Path<String>,
    autonomi_client_data: Data<Option<Client>>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data)
    );

    info!("Getting chunk at [{}]", address);
    chunk_service.get_chunk_binary(address).await
}