use actix_http::header;
use actix_web::{Error, HttpResponse};
use actix_web::error::ErrorInternalServerError;
use actix_web::http::header::{ContentLength, ContentType};
use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::controller::CacheType;
use crate::service::chunk_service::Chunk;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicData {
    content: Option<String>,
    #[schema(read_only)]
    address: Option<String>,
    #[schema(read_only)]
    cost: Option<String>,
}

pub struct PublicDataService {
    caching_client: CachingClient
}

impl PublicDataService {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn create_public_data(&self, bytes: Bytes, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<HttpResponse, Error> {
        match self.caching_client.data_put_public(bytes, PaymentOption::from(&evm_wallet), cache_only).await {
            Ok((cost, data_address)) => {
                info!("Created public data at [{}] for [{}] attos", data_address.to_hex(), cost);
                let response_data_map_chunk = Chunk::new(None, Some(data_address.to_hex()), Some(cost.to_string()));
                Ok(HttpResponse::Created().json(response_data_map_chunk))
            }
            Err(e) => {
                // todo: refine error handling to return appropriate messages / payloads
                warn!("Failed to create public data: [{:?}]", e);
                Err(ErrorInternalServerError("Failed to create public data"))
            }
        }
    }

    pub async fn get_public_data_binary(&self, address: String) -> Result<HttpResponse, Error> {
        let data_address = DataAddress::from_hex(address.as_str()).unwrap();
        match self.caching_client.data_get_public(&data_address).await {
            Ok(bytes) => {
                info!("Retrieved public data at address [{}]", address);

                // todo: add caching headers (etag, etc)
                Ok(HttpResponse::Ok()
                    .insert_header(ContentType::octet_stream())
                    .insert_header(ContentLength(bytes.len()))
                    .insert_header((header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))))
                    .body(bytes))
            }
            Err(e) => {
                warn!("Failed to retrieve public data at address [{}]: [{:?}]", address, e);
                Err(ErrorInternalServerError(format!("Failed to retrieve public data at address [{}]", address)))
            }
        }
    }
}