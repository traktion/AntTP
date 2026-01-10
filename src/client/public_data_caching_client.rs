use std::path::PathBuf;
use autonomi::client::payment::PaymentOption;
use autonomi::data::DataAddress;
use bytes::Bytes;
use chunk_streamer::chunk_encrypter::ChunkEncrypter;
use log::info;
use crate::client::CachingClient;
use crate::client::command::public_data::create_public_data_command::CreatePublicDataCommand;
use crate::error::{CreateError, GetError};
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;

impl CachingClient {

    pub async fn data_put_public(
        &self,
        data: Bytes,
        payment_option: PaymentOption,
        store_type: StoreType,
    ) -> Result<DataAddress, PublicDataError> {
        // todo: can we avoid double encrypting on upload?
        let data_address = self.cache_public_data(data.clone(), store_type.clone()).await?;
        if store_type == StoreType::Network {
            let command = Box::new(
                CreatePublicDataCommand::new(self.client_harness.clone(), data, payment_option)
            );
            self.send_create_command(command).await?;
        }
        Ok(data_address)
    }

    async fn cache_public_data(&self, data: Bytes, store_type: StoreType) -> Result<DataAddress, PublicDataError> {
        let chunk_encrypter = ChunkEncrypter::new();
        match chunk_encrypter.encrypt(true, data.clone()).await {
            Ok((chunks, data_map_chunk)) => {
                let data_map_addr = *data_map_chunk.0.address();
                info!("updating cache with data map chunk at address [{}]", data_map_addr.to_hex());
                let data_address = DataAddress::new(*data_map_addr.xorname());

                for chunk in chunks {
                    if store_type == StoreType::Disk {
                        info!("updating disk cache with chunk at address [{}]", chunk.address.to_hex());
                        self.hybrid_cache.insert(format!("{}", chunk.address.to_hex()), chunk.value.to_vec());
                    } else {
                        info!("updating cache with chunk at address [{}]", chunk.address.to_hex());
                        self.hybrid_cache.memory().insert(format!("{}", chunk.address.to_hex()), chunk.value.to_vec());
                    }
                }
                Ok(data_address)
            },
            Err(e) => Err(CreateError::Encryption(e.to_string()).into())
        }
    }

    pub async fn data_get_public(&self, addr: &DataAddress) -> Result<Bytes, PublicDataError> {
        match self.download_stream(addr, 0, 0).await {
            Ok(bytes) => {
                info!("retrieved public data for [{}] with size [{}]", addr.to_hex(), bytes.len());
                Ok(bytes)
            },
            Err(e) => Err(GetError::RecordNotFound(
                format!("Failed to download stream at address [{}] with error [{}]", addr, e.to_string())).into()),
        }
    }

    pub async fn file_content_upload_public(&self, path: PathBuf, payment_option: PaymentOption, store_type: StoreType) -> Result<DataAddress, PublicDataError> {
        match tokio::fs::read(path.clone()).await {
            Ok(vec_data) => {
                let data = Bytes::from(vec_data);
                let addr = self.data_put_public(data, payment_option.clone(), store_type).await?;
                Ok(addr)
            },
            Err(e) => Err(CreateError::TemporaryStorage(e.to_string()).into())
        }
    }
}