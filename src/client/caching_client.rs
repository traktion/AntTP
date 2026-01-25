use std::fs;
use actix_web::web::Data;
use async_job::{Job, Schedule};
use async_trait::async_trait;
use autonomi::ChunkAddress;
use autonomi::data::DataAddress;
use chunk_streamer::chunk_streamer::ChunkStreamer;
use foyer::HybridCache;
use log::error;
use crate::config::anttp_config::AntTpConfig;
use bytes::{BufMut, Bytes, BytesMut};
use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use crate::client::CachingClient;
use crate::client::chunk_caching_client::ChunkCachingClient;
use crate::client::client_harness::ClientHarness;
use crate::client::command::Command;
use crate::error::{CheckError, CreateError, GetError, GetStreamError, UpdateError};
use crate::error::chunk_error::ChunkError;


pub const ARCHIVE_TAR_IDX_BYTES: &[u8] = "\0archive.tar.idx\0".as_bytes();

#[async_trait]
impl Job for CachingClient {
    fn schedule(&self) -> Option<Schedule> {
        Some("1/10 * * * * *".parse().unwrap())
    }
    async fn handle(&mut self) {
        let mut harness = self.client_harness.get_ref().lock().await;
        harness.try_sleep();
    }
}

impl CachingClient {

    pub fn new(client_harness: Data<Mutex<ClientHarness>>, ant_tp_config: AntTpConfig,
               hybrid_cache: Data<HybridCache<String, Vec<u8>>>, command_executor: Data<Sender<Box<dyn Command>>>) -> Self {
        let cache_dir = ant_tp_config.clone().map_cache_directory;
        CachingClient::create_tmp_dir(cache_dir.clone());

        Self {
            client_harness, ant_tp_config, hybrid_cache, command_executor
        }
    }

    fn create_tmp_dir(cache_dir: String) {
        if !fs::exists(cache_dir.clone()).unwrap() {
            fs::create_dir_all(cache_dir.clone()).unwrap_or_default()
        }
    }

    pub async fn download_stream(
        &self,
        addr: &DataAddress,
        range_from: i64,
        range_to: i64,
    ) -> Result<Bytes, ChunkError> {
        // todo: combine with file_service code
        match ChunkCachingClient::new(self.clone()).chunk_get_internal(&ChunkAddress::new(*addr.xorname())).await {
            Ok(data_map_chunk) => {
                let chunk_streamer = ChunkStreamer::new(addr.to_hex(), data_map_chunk.value, self.clone(), self.ant_tp_config.download_threads);
                // only retrieve the size when it is needed
                let length = if range_from < 0 || range_to <= 0 { u64::try_from(chunk_streamer.get_stream_size().await).unwrap() } else { 0 };

                let derived_range_from = if range_from < 0 {
                    let from = u64::try_from(range_from.abs()).unwrap();
                    if from < length {
                        length.saturating_sub(1).saturating_sub(from)
                    } else {
                        0
                    }
                } else {
                    u64::try_from(range_from).unwrap()
                };
                let derived_range_to: u64 = if range_to <= 0 {
                    let to = u64::try_from(range_to.abs()).unwrap();
                    if to < length {
                        length.saturating_sub(1).saturating_sub(to)
                    } else {
                        0
                    }
                } else {
                    let to = u64::try_from(range_to).unwrap();
                    if to > length.saturating_sub(1) {
                        length
                    } else {
                        to
                    }
                };

                let mut chunk_receiver: chunk_streamer::chunk_receiver::ChunkReceiver = match chunk_streamer.open(derived_range_from, derived_range_to).await {
                    Ok(chunk_receiver) => chunk_receiver,
                    Err(e) => return Err(ChunkError::GetStreamError(GetStreamError::BadReceiver(format!("failed to open chunk stream: {}", e)))),
                };

                let mut buf = BytesMut::with_capacity(usize::try_from(derived_range_to - derived_range_from).expect("Failed to convert range from u64 to usize"));
                let mut has_data = true;
                while has_data {
                    match chunk_receiver.next().await {
                        Some(item) => match item {
                            Ok(bytes) => buf.put(bytes),
                            Err(e) => {
                                error!("Error downloading stream from data address [{}] with range [{} - {}]: {}", addr.to_hex(), derived_range_from, derived_range_to, e);
                                has_data = false
                            },
                        },
                        None => has_data = false
                    };
                }
                Ok(buf.freeze())
            }
            Err(e) => Err(e)
        }
    }

    pub async fn send_create_command(&self, command: Box<dyn Command>) -> Result<(), CreateError> {
        Ok(self.command_executor.send(command).await?)
    }

    pub async fn send_update_command(&self, command: Box<dyn Command>) -> Result<(), UpdateError> {
        Ok(self.command_executor.send(command).await?)
    }

    pub async fn send_get_command(&self, command: Box<dyn Command>) -> Result<(), GetError> {
        Ok(self.command_executor.send(command).await?)
    }

    pub async fn send_check_command(&self, command: Box<dyn Command>) -> Result<(), CheckError> {
        Ok(self.command_executor.send(command).await?)
    }
}

#[cfg(test)]
mod tests {
    /// Test range calculation with zero-length data to ensure no overflow
    #[test]
    fn test_range_calculation_with_zero_length() {
        let length: u64 = 0;

        // Test negative range_from with zero length
        let from = 1u64;
        let derived_range_from = if from < length {
            length.saturating_sub(1).saturating_sub(from)
        } else {
            0
        };
        assert_eq!(derived_range_from, 0);

        // Test negative range_to with zero length
        let to = 1u64;
        let derived_range_to = if to < length {
            length.saturating_sub(1).saturating_sub(to)
        } else {
            0
        };
        assert_eq!(derived_range_to, 0);

        // Test positive range_to exceeding length
        let to = 100u64;
        let derived_range_to = if to > length.saturating_sub(1) {
            length
        } else {
            to
        };
        assert_eq!(derived_range_to, 0);
    }

    /// Test range calculation with normal data length
    #[test]
    fn test_range_calculation_with_normal_length() {
        let length: u64 = 100;

        // Test negative range_from (last 10 bytes)
        let from = 10u64;
        let derived_range_from = if from < length {
            length.saturating_sub(1).saturating_sub(from)
        } else {
            0
        };
        assert_eq!(derived_range_from, 89); // 99 - 10 = 89

        // Test negative range_to (excluding last 5 bytes)
        let to = 5u64;
        let derived_range_to = if to < length {
            length.saturating_sub(1).saturating_sub(to)
        } else {
            0
        };
        assert_eq!(derived_range_to, 94); // 99 - 5 = 94

        // Test positive range_to within bounds
        let to = 50u64;
        let derived_range_to = if to > length.saturating_sub(1) {
            length
        } else {
            to
        };
        assert_eq!(derived_range_to, 50);

        // Test positive range_to exceeding bounds
        let to = 150u64;
        let derived_range_to = if to > length.saturating_sub(1) {
            length
        } else {
            to
        };
        assert_eq!(derived_range_to, 100);
    }

    /// Test edge case: from exceeds length
    #[test]
    fn test_range_calculation_from_exceeds_length() {
        let length: u64 = 10;

        let from = 20u64;
        let derived_range_from = if from < length {
            length.saturating_sub(1).saturating_sub(from)
        } else {
            0
        };
        assert_eq!(derived_range_from, 0);
    }
}