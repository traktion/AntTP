use autonomi::client::payment::PaymentOption;
use autonomi::Wallet;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{info};
use mockall_double::double;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[double]
use crate::client::PublicDataCachingClient;
use crate::error::GetError;
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;
use crate::service::chunk_service::Chunk;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicData {
    #[schema(read_only)]
    address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PublicDataService {
    public_data_caching_client: PublicDataCachingClient
}

impl PublicDataService {
    pub fn new(public_data_caching_client: PublicDataCachingClient) -> Self {
        Self { public_data_caching_client }
    }

    pub async fn create_public_data(&self, bytes: Bytes, evm_wallet: Wallet, store_type: StoreType) -> Result<Chunk, PublicDataError> {
        let data_address: DataAddress = self.public_data_caching_client.data_put_public(bytes, PaymentOption::from(&evm_wallet), store_type).await?;
        info!("Queued command to create public data at [{}]", data_address.to_hex());
        Ok(Chunk::new(None, Some(data_address.to_hex())))
    }

    pub async fn get_public_data_binary(&self, address: String) -> Result<Bytes, PublicDataError> {
        match DataAddress::from_hex(address.as_str()) {
            Ok(data_address) => self.public_data_caching_client.data_get_public(&data_address).await,
            Err(e) => Err(PublicDataError::GetError(GetError::BadAddress(e.to_string())))
        }
    }
}