use std::fs;
use actix_web::web::Data;
use ant_protocol::storage::Chunk;
use async_job::{Job, Schedule};
use async_trait::async_trait;
use autonomi::ChunkAddress;
use autonomi::data::DataAddress;
use chunk_streamer::chunk_streamer::ChunkStreamer;
use foyer::HybridCache;
use log::{debug, error};
use crate::config::anttp_config::AntTpConfig;
use bytes::{BufMut, Bytes, BytesMut};
use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;
#[double]
use crate::client::ChunkCachingClient;
use crate::error::{CheckError, CreateError, GetError, GetStreamError, UpdateError};
use crate::error::chunk_error::ChunkError;
use chunk_streamer::chunk_streamer::ChunkGetter;
use mockall::mock;
use mockall_double::double;
use crate::client::client_harness::ClientHarness;
use crate::client::command::Command;

#[derive(Debug, Clone)]
pub struct CachingClient {
    pub client_harness: Data<tokio::sync::Mutex<ClientHarness>>,
    pub ant_tp_config: AntTpConfig,
    pub hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    pub command_executor: Data<Sender<Box<dyn Command>>>,
}

mock! {
    #[derive(Debug)]
    pub CachingClient {
        pub fn new(client_harness: Data<tokio::sync::Mutex<ClientHarness>>, ant_tp_config: AntTpConfig,
                   hybrid_cache: Data<HybridCache<String, Vec<u8>>>, command_executor: Data<Sender<Box<dyn Command>>>) -> Self;
        pub async fn download_stream(
            &self,
            addr: &DataAddress,
            range_from: i64,
            range_to: i64,
        ) -> Result<Bytes, ChunkError>;
        pub fn get_derived_ranges(&self, range_from: i64, range_to: i64, length: Option<u64>) -> (u64, u64);
        pub async fn send_create_command(&self, command: Box<dyn Command>) -> Result<(), CreateError>;
        pub async fn send_update_command(&self, command: Box<dyn Command>) -> Result<(), UpdateError>;
        pub async fn send_get_command(&self, command: Box<dyn Command>) -> Result<(), GetError>;
        pub async fn send_check_command(&self, command: Box<dyn Command>) -> Result<(), CheckError>;
        pub fn get_hybrid_cache(&self) -> &Data<HybridCache<String, Vec<u8>>>;
        pub fn get_client_harness(&self) -> &Data<tokio::sync::Mutex<ClientHarness>>;
        pub fn get_ant_tp_config(&self) -> &AntTpConfig;
    }
    impl Clone for CachingClient {
        fn clone(&self) -> Self;
    }
    #[async_trait]
    impl ChunkGetter for CachingClient {
        async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, autonomi::client::GetError>;
    }
}

/*#[async_trait]
impl ChunkGetter for CachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, autonomi::client::GetError> {
        self.chunk_get(address).await
    }
}*/

#[async_trait]
impl ChunkGetter for CachingClient {
    async fn chunk_get(&self, address: &ChunkAddress) -> Result<Chunk, autonomi::client::GetError> {
        let chunk_caching_client = crate::client::ChunkCachingClient::new(self.clone());
        chunk_caching_client.chunk_get(address).await
    }
}

pub const ARCHIVE_TAR_IDX_BYTES: &[u8] = "\0archive.tar.idx\0".as_bytes();

#[cfg(not(test))]
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

    pub fn new(client_harness: Data<tokio::sync::Mutex<ClientHarness>>, ant_tp_config: AntTpConfig,
               hybrid_cache: Data<HybridCache<String, Vec<u8>>>, command_executor: Data<Sender<Box<dyn Command>>>) -> Self {
        let cache_dir = ant_tp_config.clone().map_cache_directory;
        CachingClient::create_tmp_dir(cache_dir.clone());

        Self {
            client_harness, ant_tp_config, hybrid_cache, command_executor
        }
    }

    pub fn get_hybrid_cache(&self) -> &Data<HybridCache<String, Vec<u8>>> {
        &self.hybrid_cache
    }

    pub fn get_client_harness(&self) -> &Data<tokio::sync::Mutex<ClientHarness>> {
        &self.client_harness
    }

    pub fn get_ant_tp_config(&self) -> &AntTpConfig {
        &self.ant_tp_config
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
                    addr, range_from, range_to, derived_range_from, derived_range_to);
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
#[async_trait]
impl Job for CachingClient {
    fn schedule(&self) -> Option<Schedule> {
        Some("1/10 * * * * *".parse().unwrap())
    }
    async fn handle(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use ant_evm::EvmNetwork;
    use foyer::HybridCacheBuilder;
    use clap::Parser;
    use mockall_double::double;
    use tokio::sync::mpsc;
    use tempfile::tempdir;

    async fn create_mock_caching_client() -> (CachingClient, mpsc::Receiver<Box<dyn Command>>) {
        let (tx, rx) = mpsc::channel(100);
        let ant_tp_config = AntTpConfig::parse_from(&[
            "anttp",
            "--map-cache-directory",
            tempdir().unwrap().path().to_str().unwrap()
        ]);

        let client_harness = ClientHarness::new(EvmNetwork::ArbitrumOne, ant_tp_config.clone());
        let hybrid_cache = HybridCacheBuilder::new().memory(1024).storage().build().await.unwrap();

        let client = CachingClient::new(
            Data::new(tokio::sync::Mutex::new(client_harness)),
            ant_tp_config,
            Data::new(hybrid_cache),
            Data::new(tx),
        );

        (client, rx)
    }

    #[tokio::test]
    async fn test_get_derived_ranges_with_length() {
        let (client, _) = create_mock_caching_client().await;
        let length = Some(100u64);

        // Positive ranges within bounds
        assert_eq!(client.get_derived_ranges(10, 50, length), (10, 50));

        // Positive range_to exceeding length
        assert_eq!(client.get_derived_ranges(10, 150, length), (10, 99));

        // Positive range_from exceeding length
        assert_eq!(client.get_derived_ranges(150, 200, length), (99, 99));

        // Negative range_from (from the end)
        // range_from = -10 means last 10 bytes: from 89 to 99 (length 100)
        assert_eq!(client.get_derived_ranges(-10, 100, length), (89, 99));

        // Negative range_to (excluding from the end)
        // range_to = -5: length - 1 - 5 = 94.
        assert_eq!(client.get_derived_ranges(0, -5, length), (0, 94));

        // Both negative
        // range_from = -10 (89), range_to = -5 (94)
        assert_eq!(client.get_derived_ranges(-10, -5, length), (89, 94));
    }

    #[tokio::test]
    async fn test_get_derived_ranges_without_length() {
        let (client, _) = create_mock_caching_client().await;

        // Should just return absolute values
        assert_eq!(client.get_derived_ranges(10, 50, None), (10, 50));
        assert_eq!(client.get_derived_ranges(-10, -50, None), (10, 50));
    }

    #[tokio::test]
    async fn test_get_derived_ranges_zero_length() {
        let (client, _) = create_mock_caching_client().await;
        let length = Some(0u64);

        assert_eq!(client.get_derived_ranges(0, 10, length), (0, 0));
        assert_eq!(client.get_derived_ranges(-1, -1, length), (0, 0));
    }

    #[tokio::test]
    async fn test_new_creates_cache_dir() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("test_cache");
        let cache_dir_str = cache_path.to_str().unwrap().to_string();

        let ant_tp_config = AntTpConfig::parse_from(&[
            "anttp",
            "--map-cache-directory",
            &cache_dir_str
        ]);

        let (tx, _) = mpsc::channel(1);
        let client_harness = ClientHarness::new(EvmNetwork::ArbitrumOne, ant_tp_config.clone());
        let hybrid_cache = HybridCacheBuilder::new().memory(1024).storage().build().await.unwrap();

        let _client = CachingClient::new(
            Data::new(tokio::sync::Mutex::new(client_harness)),
            ant_tp_config,
            Data::new(hybrid_cache),
            Data::new(tx),
        );

        assert!(cache_path.exists());
    }

    #[tokio::test]
    async fn test_job_schedule() {
        let (client, _) = create_mock_caching_client().await;
        assert!(client.schedule().is_some());
    }

    #[tokio::test]
    async fn test_job_handle() {
        let (mut client, _) = create_mock_caching_client().await;
        // This should not panic and should at least lock the harness
        client.handle().await;
    }

    #[tokio::test]
    async fn test_send_commands() {
        let (client, mut rx) = create_mock_caching_client().await;

        struct MockCommand;
        #[async_trait]
        impl Command for MockCommand {
            async fn execute(&self) -> Result<(), crate::client::command::error::CommandError> {
                Ok(())
            }
            fn action_hash(&self) -> Vec<u8> { vec![] }
            fn id(&self) -> u128 { 0 }
        }

        // Test send_create_command
        let res = client.send_create_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());
        let _received: Box<dyn Command> = rx.recv().await.unwrap();

        // Test send_update_command
        let res = client.send_update_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());
        let _received: Box<dyn Command> = rx.recv().await.unwrap();

        // Test send_get_command
        let res = client.send_get_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());
        let _received: Box<dyn Command> = rx.recv().await.unwrap();

        // Test send_check_command
        let res = client.send_check_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());
        let _received: Box<dyn Command> = rx.recv().await.unwrap();
    }
}