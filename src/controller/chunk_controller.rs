use actix_web::{web, Responder};
use actix_web::error::ErrorInternalServerError;
use actix_web::web::{Data, Payload};
use ant_evm::EvmWallet;
use autonomi::Client;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::ClientCacheState;
use crate::config::anttp_config::AntTpConfig;
use crate::service::chunk_service::{Chunk, ChunkService};

pub async fn post_chunk(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    chunk: web::Json<Chunk>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state)
    );

    info!("Creating new chunk");
    chunk_service.create_chunk(chunk.into_inner(), evm_wallet).await
}

pub async fn post_chunk_binary(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    payload: Payload,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state)
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

pub async fn get_chunk(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state)
    );

    info!("Getting chunk at [{}]", address);
    chunk_service.get_chunk(address).await
}

pub async fn get_chunk_binary(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let chunk_service = ChunkService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state)
    );

    info!("Getting chunk at [{}]", address);
    chunk_service.get_chunk_binary(address).await
}