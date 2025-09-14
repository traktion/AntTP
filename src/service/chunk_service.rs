use actix_http::header;
use actix_web::{Error, HttpResponse};
use actix_web::error::{ErrorInternalServerError};
use actix_web::http::header::{ContentLength, ContentType};
use autonomi::{ChunkAddress, Wallet};
use autonomi::client::chunk as autonomi_chunk;
use autonomi::client::payment::PaymentOption;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::{Bytes};
use chunk_streamer::chunk_streamer::ChunkGetter;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Chunk {
    content: Option<String>,
    #[schema(read_only)]
    address: Option<String>,
    #[schema(read_only)]
    cost: Option<String>,
}

impl Chunk {
    pub fn new(content: Option<String>, address: Option<String>, cost: Option<String>) -> Self {
        Chunk { content, address, cost }
    }
}

pub struct ChunkService {
    caching_client: CachingClient
}

impl ChunkService {

    pub fn new(caching_client: CachingClient) -> Self {
        ChunkService { caching_client }
    }

    pub async fn create_chunk_binary(&self, bytes: Bytes, evm_wallet: Wallet, is_cache_only: bool) -> Result<HttpResponse, Error> {
        let chunk_data =  autonomi_chunk::Chunk::new(bytes);
        self.create_chunk_raw(chunk_data, evm_wallet, is_cache_only).await
    }

    pub async fn create_chunk(&self, chunk: Chunk, evm_wallet: Wallet, is_cache_only: bool) -> Result<HttpResponse, Error> {
        let content = match chunk.content.clone() {
            Some(content) => content,
            None => return Err(ErrorInternalServerError("Empty chunk payload"))
        };
        let decoded_content = BASE64_STANDARD.decode(content).unwrap_or_else(|_| Vec::new());
        let chunk_data =  autonomi_chunk::Chunk::new(Bytes::from(decoded_content.clone()));

        self.create_chunk_raw(chunk_data, evm_wallet, is_cache_only).await
    }

    pub async fn create_chunk_raw(&self, chunk: autonomi_chunk::Chunk, evm_wallet: Wallet, is_cache_only: bool) -> Result<HttpResponse, Error> {
        info!("Create chunk at address [{}]", chunk.address.to_hex());
        match self.caching_client.chunk_put(&chunk, PaymentOption::from(&evm_wallet), is_cache_only).await {
            Ok((cost, chunk_address)) => {
                info!("Created chunk at [{}] for [{}] attos", chunk_address.to_hex(), cost);
                let response_chunk = Chunk::new(None, Some(chunk_address.to_hex()), Some(cost.to_string()));
                Ok(HttpResponse::Created().json(response_chunk))
            }
            Err(e) => {
                // todo: refine error handling to return appropriate messages / payloads
                warn!("Failed to create chunk: [{:?}]", e);
                Err(ErrorInternalServerError("Failed to create chunk"))
            }
        }
    }

    pub async fn get_chunk_binary(&self, address: String) -> Result<HttpResponse, Error> {
        let chunk_address = ChunkAddress::from_hex(address.as_str()).unwrap();
        match self.caching_client.chunk_get(&chunk_address).await {
            Ok(chunk) => {
                info!("Retrieved chunk at address [{}]", address);

                // todo: add caching headers (etag, etc)
                Ok(HttpResponse::Ok()
                    .insert_header(ContentType::octet_stream())
                    .insert_header(ContentLength(chunk.size()))
                    .insert_header((header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))))
                    .body(chunk.value))
            }
            Err(e) => {
                warn!("Failed to retrieve chunk at address [{}]: [{:?}]", address, e);
                Err(ErrorInternalServerError(format!("Failed to retrieve chunk at address [{}]", address)))
            }
        }
    }

    pub async fn get_chunk(&self, address: String) -> Result<HttpResponse, Error> {
        let chunk_address = ChunkAddress::from_hex(address.as_str()).unwrap();
        match self.caching_client.chunk_get(&chunk_address).await {
            Ok(chunk) => {
                info!("Retrieved chunk at address [{}]", address);
                let encoded_chunk = BASE64_STANDARD.encode(chunk.value);
                let response_chunk = Chunk::new(Some(encoded_chunk), Some(address), None);
                Ok(HttpResponse::Ok().json(response_chunk).into())
            }
            Err(e) => {
                warn!("Failed to retrieve chunk at address [{}]: [{:?}]", address, e);
                Err(ErrorInternalServerError(format!("Failed to retrieve chunk at address [{}]", address)))
            }
        }
    }
}