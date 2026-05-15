use std::path::PathBuf;
use ant_core::data::XorName;
use bytes::Bytes;
use chunk_streamer::chunk_encrypter::ChunkEncrypter;
use hex::ToHex;
use log::info;
use mockall::mock;
use mockall_double::double;
#[double]
use crate::client::CachingClient;
use crate::client::command::public_data::create_public_data_command::CreatePublicDataCommand;
#[double]
use crate::client::StreamingClient;
use crate::error::{CreateError, GetError};
use crate::controller::StoreType;
use crate::error::public_data_error::PublicDataError;

#[derive(Clone)]
pub struct PublicDataCachingClient {
    caching_client: CachingClient,
    streaming_client: StreamingClient,
}

mock! {
    pub PublicDataCachingClient {
        pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self;
        pub async fn data_put_public(
            &self,
            data: Bytes,
            store_type: StoreType,
        ) -> Result<XorName, PublicDataError>;
        pub async fn data_get_public(&self, addr: &XorName) -> Result<Bytes, PublicDataError>;
        pub async fn file_content_upload_public(&self, path: PathBuf, store_type: StoreType) -> Result<XorName, PublicDataError>;
    }
    impl Clone for PublicDataCachingClient {
        fn clone(&self) -> Self;
    }
}

impl PublicDataCachingClient {
    pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self {
        Self { caching_client, streaming_client }
    }

    pub async fn data_put_public(
        &self,
        data: Bytes,
        store_type: StoreType,
    ) -> Result<XorName, PublicDataError> {
        // todo: can we avoid double encrypting on upload?
        let data_address = self.cache_public_data(data.clone(), store_type.clone()).await?;
        if store_type == StoreType::Network {
            let command = Box::new(
                CreatePublicDataCommand::new(self.caching_client.get_client_harness().clone(), data)
            );
            self.caching_client.send_create_command(command).await?;
        }
        Ok(data_address)
    }

    async fn cache_public_data(&self, data: Bytes, store_type: StoreType) -> Result<XorName, PublicDataError> {
        let chunk_encrypter = ChunkEncrypter::new();
        match chunk_encrypter.encrypt(true, data.clone()).await {
            Ok((chunks, data_map_chunk)) => {
                let data_map_addr = data_map_chunk.chunk_identifiers.get(0).unwrap().dst_hash.0;
                info!("updating cache with data map chunk at address [{}]", hex::encode(data_map_addr));

                for chunk_info in data_map_chunk.chunk_identifiers {
                    match chunks.get(chunk_info.index) {
                        Some(chunk) => {
                            let chunk_address: String = chunk_info.dst_hash.encode_hex();
                            let chunk_content = chunk.content.to_vec();
                            if store_type == StoreType::Disk {
                                info!("updating disk cache with chunk at address [{}]", chunk_address);
                                self.caching_client.get_hybrid_cache().insert(format!("{}", chunk_address), chunk_content);
                            } else {
                                info!("updating cache with chunk at address [{}]", chunk_address);
                                self.caching_client.get_hybrid_cache().memory().insert(format!("{}", chunk_address), chunk_content);
                            }
                        },
                        None => {
                            return Err(CreateError::Encryption("Failed to encrypt public data".to_string()).into())
                        }
                    }
                }
                Ok(XorName::from(data_map_addr))
            },
            Err(e) => Err(CreateError::Encryption(e.to_string()).into())
        }
    }

    pub async fn data_get_public(&self, addr: &XorName) -> Result<Bytes, PublicDataError> {
        let addr_hex: String = addr.encode_hex();
        match self.streaming_client.download_stream(addr, 0, 0).await {
            Ok(bytes) => {
                info!("retrieved public data for [{}] with size [{}]", addr_hex, bytes.len());
                Ok(bytes)
            },
            Err(e) => Err(GetError::RecordNotFound(
                format!("Failed to download stream at address [{}] with error [{}]", addr_hex, e.to_string())).into()),
        }
    }

    pub async fn file_content_upload_public(&self, path: PathBuf, store_type: StoreType) -> Result<XorName, PublicDataError> {
        match tokio::fs::read(path.clone()).await {
            Ok(vec_data) => {
                let data = Bytes::from(vec_data);
                let addr = self.data_put_public(data, store_type).await?;
                Ok(addr)
            },
            Err(e) => Err(CreateError::TemporaryStorage(e.to_string()).into())
        }
    }
}