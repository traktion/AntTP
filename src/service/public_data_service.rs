use actix_web::{Error, HttpResponse};
use actix_web::error::ErrorInternalServerError;
use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::client::error::{GetError, PublicDataError};
use crate::controller::CacheType;
use crate::service::chunk_service::Chunk;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicData {
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

    pub async fn get_public_data_binary(&self, address: String) -> Result<Bytes, PublicDataError> {
        match DataAddress::from_hex(address.as_str()) {
            Ok(data_address) => self.caching_client.data_get_public(&data_address).await,
            Err(e) => Err(PublicDataError::GetError(GetError::BadAddress(e.to_string())))
        }
    }
}