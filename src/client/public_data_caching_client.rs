use std::path::PathBuf;
use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::client::PutError;
use autonomi::data::DataAddress;
use autonomi::files::{UploadError};
use bytes::Bytes;
use chunk_streamer::chunk_encrypter::ChunkEncrypter;
use log::{info};
use crate::client::CachingClient;
use crate::client::command::public_data::create_public_data_command::CreatePublicDataCommand;
use crate::client::error::{GetError, PublicDataError};
use crate::controller::CacheType;

impl CachingClient {

    pub async fn data_put_public(
        &self,
        data: Bytes,
        payment_option: PaymentOption,
        cache_only: Option<CacheType>,
    ) -> Result<(AttoTokens, DataAddress), PutError> {
        // todo: can we avoid double encrypting on upload?
        match self.cache_public_data(data.clone(), cache_only.clone()).await {
            Ok(data_address) => {
                if !cache_only.is_some() {
                    self.command_executor.send(
                        Box::new(CreatePublicDataCommand::new(self.client_harness.clone(), data, payment_option))
                    ).await.unwrap();
                }
                Ok((AttoTokens::zero(), data_address))
            },
            Err(err) => Err(err)
        }
    }

    async fn cache_public_data(&self, data: Bytes, cache_only: Option<CacheType>) -> Result<DataAddress, PutError> {
        let chunk_encrypter = ChunkEncrypter::new();
        match chunk_encrypter.encrypt(true, data.clone()).await {
            Ok((chunks, data_map_chunk)) => {
                let data_map_addr = *data_map_chunk.0.address();
                info!("updating cache with data map chunk at address [{}]", data_map_addr.to_hex());
                let data_address = DataAddress::new(*data_map_addr.xorname());

                for chunk in chunks {
                    if cache_only.clone().is_some_and(|v| matches!(v, CacheType::Disk)) {
                        info!("updating disk cache with chunk at address [{}]", chunk.address.to_hex());
                        self.hybrid_cache.insert(format!("{}", chunk.address.to_hex()), chunk.value.to_vec());
                    } else {
                        info!("updating cache with chunk at address [{}]", chunk.address.to_hex());
                        self.hybrid_cache.memory().insert(format!("{}", chunk.address.to_hex()), chunk.value.to_vec());
                    }
                }
                Ok(data_address)
            },
            Err(err) => Err(err)
        }
    }

    pub async fn data_get_public(&self, addr: &DataAddress) -> Result<Bytes, PublicDataError> {
        match self.download_stream(addr, 0, 0).await {
            Ok(bytes) => {
                info!("retrieved public data for [{}] with size [{}]", addr.to_hex(), bytes.len());
                Ok(bytes)
            },
            Err(e) => Err(PublicDataError::GetError(GetError::RecordNotFound(
                format!("Failed to download stream at address [{}] with error [{}]", addr, e.to_string())))),
        }
    }

    pub async fn file_content_upload_public(&self, path: PathBuf, payment_option: PaymentOption, cache_only: Option<CacheType>) -> Result<(AttoTokens, DataAddress), UploadError> {
        let data = tokio::fs::read(path.clone()).await?;
        let data = Bytes::from(data);
        let (cost, addr) = self.data_put_public(data, payment_option.clone(), cache_only).await?;
        Ok((cost, addr))
    }
}