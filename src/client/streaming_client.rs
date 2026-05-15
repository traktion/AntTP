use ant_core::data::XorName;
use bytes::{BufMut, Bytes, BytesMut};
use chunk_streamer::chunk_streamer::{ChunkGetter, ChunkStreamer};
use futures_util::StreamExt;
use hex::ToHex;
use log::{debug, error};
use mockall::mock;
use crate::client::ChunkCachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::error::chunk_error::ChunkError;
use crate::error::GetStreamError;

#[derive(Clone)]
pub struct StreamingClient {
    chunk_caching_client: ChunkCachingClient,
    ant_tp_config: AntTpConfig,
}

mock! {
    pub StreamingClient {
        pub fn new(chunk_caching_client: ChunkCachingClient, ant_tp_config: AntTpConfig) -> Self;
        pub async fn download_stream(
            &self,
            addr: &XorName,
            range_from: i64,
            range_to: i64,
        ) -> Result<Bytes, ChunkError>;
        pub fn get_derived_ranges(&self, range_from: i64, range_to: i64, length: Option<u64>) -> (u64, u64);
    }
    impl Clone for StreamingClient {
        fn clone(&self) -> Self;
    }
}

impl StreamingClient {
    pub fn new(chunk_caching_client: ChunkCachingClient, ant_tp_config: AntTpConfig) -> Self {
        Self { chunk_caching_client, ant_tp_config }
    }

    pub async fn download_stream(
        &self,
        addr: &XorName,
        range_from: i64,
        range_to: i64,
    ) -> Result<Bytes, ChunkError> {
        // todo: combine with file_service code
        let addr_hex: String = addr.encode_hex();
        match self.chunk_caching_client.chunk_get(addr).await {
            Ok(maybe_data_map_chunk) => {
                match maybe_data_map_chunk {
                    Some(data_map_chunk) => {
                        let chunk_streamer = ChunkStreamer::new(addr.encode_hex(), data_map_chunk.content, self.chunk_caching_client.clone(), self.ant_tp_config.download_threads);

                        // only retrieve the size when it is needed
                        let length = if range_from < 0 || range_to <= 0 {
                            u64::try_from(chunk_streamer.get_stream_size().await).ok()
                        } else {
                            None
                        };

                        let (derived_range_from, derived_range_to) = self.get_derived_ranges(range_from, range_to, length);

                        let mut chunk_receiver: chunk_streamer::chunk_receiver::ChunkReceiver = match chunk_streamer.open(derived_range_from, derived_range_to).await {
                            Ok(chunk_receiver) => chunk_receiver,
                            Err(e) => return Err(ChunkError::GetStreamError(GetStreamError::BadReceiver(format!("failed to open chunk stream: {}", e)))),
                        };

                        debug!("streaming from addr [{}], range_from: [{}], range_to: [{}], derived_range_from: [{}], derived_range_to: [{}]",
                            addr_hex, range_from, range_to, derived_range_from, derived_range_to);
                        let mut buf = BytesMut::with_capacity(usize::try_from(derived_range_to - derived_range_from).expect("Failed to convert range from u64 to usize"));
                        let mut has_data = true;
                        while has_data {
                            match chunk_receiver.next().await {
                                Some(item) => match item {
                                    Ok(bytes) => buf.put(bytes),
                                    Err(e) => {
                                        error!("Error downloading stream from data address [{}] with range [{} - {}]: {}", addr_hex, derived_range_from, derived_range_to, e);
                                        has_data = false
                                    },
                                },
                                None => has_data = false
                            };
                        }
                        Ok(buf.freeze())       
                    }
                    None => {
                        Err(ChunkError::GetStreamError(GetStreamError::BadReceiver(format!("failed to get data map chunk for data address: {}", addr_hex))))
                    }
                }
            }
            Err(e) => Err(ChunkError::GetStreamError(GetStreamError::BadReceiver(format!("failed to get_chunk for data address: {}", e))))
        }
    }

    pub fn get_derived_ranges(&self, range_from: i64, range_to: i64, length: Option<u64>) -> (u64, u64) {
        match length {
            Some(length) => {
                let derived_range_from: u64 = if range_from < 0 {
                    let from = u64::try_from(range_from.abs()).unwrap();
                    if from < length {
                        length.saturating_sub(1).saturating_sub(from)
                    } else {
                        0 // start from the beginning
                    }
                } else {
                    let from = u64::try_from(range_from).unwrap();
                    if from > length.saturating_sub(1) {
                        length.saturating_sub(1)
                    } else {
                        from
                    }
                };
                let derived_range_to: u64 = if range_to <= 0 {
                    let to = u64::try_from(range_to.abs()).unwrap();
                    if to < length {
                        length.saturating_sub(1).saturating_sub(to)
                    } else {
                        length.saturating_sub(1)
                    }
                } else {
                    let to = u64::try_from(range_to).unwrap();
                    if to > length.saturating_sub(1) {
                        length.saturating_sub(1)
                    } else {
                        to
                    }
                };
                (derived_range_from, derived_range_to)
            },
            None => {
                let derived_range_from = u64::try_from(range_from.abs()).unwrap();
                let derived_range_to = u64::try_from(range_to.abs()).unwrap();
                (derived_range_from, derived_range_to)
            }
        }
    }
}