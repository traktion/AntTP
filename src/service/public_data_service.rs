use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{info};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::client::CachingClient;
use crate::error::GetError;
use crate::controller::CacheType;
use crate::error::public_data_error::PublicDataError;
use crate::service::chunk_service::Chunk;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicData {
    #[schema(read_only)]
    address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PublicDataService {
    caching_client: CachingClient
}

impl PublicDataService {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn create_public_data(&self, bytes: Bytes, evm_wallet: Wallet, cache_only: Option<CacheType>) -> Result<Chunk, PublicDataError> {
        let data_address = self.caching_client.data_put_public(bytes, PaymentOption::from(&evm_wallet), cache_only).await?;
        info!("Queued command to create public data at [{}]", data_address.to_hex());
        Ok(Chunk::new(None, Some(data_address.to_hex())))
    }

    pub async fn get_public_data_binary(&self, address: String) -> Result<Bytes, PublicDataError> {
        match DataAddress::from_hex(address.as_str()) {
            Ok(data_address) => self.caching_client.data_get_public(&data_address).await,
            Err(e) => Err(PublicDataError::GetError(GetError::BadAddress(e.to_string())))
        }
    }
}