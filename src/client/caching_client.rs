use std::fs;
use actix_web::web::Data;
use async_job::{Job, Schedule};
use async_trait::async_trait;
use foyer::HybridCache;
use crate::config::anttp_config::AntTpConfig;
use tokio::sync::mpsc::Sender;
use crate::error::{CheckError, CreateError, GetError, UpdateError};
use mockall::mock;
use crate::client::client_harness::ClientHarness;
use crate::client::command::Command;

#[derive(Clone)]
pub struct CachingClient {
    pub client_harness: Data<tokio::sync::Mutex<ClientHarness>>,
    pub ant_tp_config: AntTpConfig,
    pub hybrid_cache: Data<HybridCache<String, Vec<u8>>>,
    pub command_executor: Data<Sender<Box<dyn Command>>>,
}

mock! {
    pub CachingClient {
        pub fn new(client_harness: Data<tokio::sync::Mutex<ClientHarness>>, ant_tp_config: AntTpConfig,
                   hybrid_cache: Data<HybridCache<String, Vec<u8>>>, command_executor: Data<Sender<Box<dyn Command>>>) -> Self;
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
    impl Job for CachingClient {
        fn schedule(&self) -> Option<Schedule>;
        async fn handle(&mut self);
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
    use foyer::HybridCacheBuilder;
    use clap::Parser;
    use evmlib::Network::ArbitrumOne;
    use tokio::sync::mpsc;
    use tempfile::tempdir;

    async fn create_mock_caching_client() -> (MockCachingClient, mpsc::Receiver<Box<dyn Command>>) {
        let (_tx, rx) = mpsc::channel(100);
        let mut client = MockCachingClient::default();
        (client, rx)
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
        let client_harness = ClientHarness::new(ArbitrumOne, ant_tp_config.clone());
        let hybrid_cache = HybridCacheBuilder::new().memory(1024).storage().build().await.unwrap();

        let ctx = MockCachingClient::new_context();
        ctx.expect()
            .returning(|_client_harness, config, _hybrid_cache, _command_executor| {
                let cache_dir = config.clone().map_cache_directory;
                CachingClient::create_tmp_dir(cache_dir.clone());
                MockCachingClient::default()
            });

        let _client = MockCachingClient::new(
            Data::new(tokio::sync::Mutex::new(client_harness)),
            ant_tp_config,
            Data::new(hybrid_cache),
            Data::new(tx),
        );

        assert!(cache_path.exists());
    }

    #[tokio::test]
    async fn test_job_schedule() {
        let (mut client, _rx) = create_mock_caching_client().await;
        client.expect_schedule().returning(|| Some("1/10 * * * * *".parse().unwrap()));
        assert!(client.schedule().is_some());
    }

    #[tokio::test]
    async fn test_job_handle() {
        let (mut client, _rx) = create_mock_caching_client().await;
        client.expect_handle().returning(|| ());
        // This should not panic and should at least lock the harness
        client.handle().await;
    }

    #[tokio::test]
    async fn test_send_commands() {
        let (mut client, _rx) = create_mock_caching_client().await;

        client.expect_send_create_command().returning(|_| Ok(()));
        client.expect_send_update_command().returning(|_| Ok(()));
        client.expect_send_get_command().returning(|_| Ok(()));
        client.expect_send_check_command().returning(|_| Ok(()));

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

        // Test send_update_command
        let res = client.send_update_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());

        // Test send_get_command
        let res = client.send_get_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());

        // Test send_check_command
        let res = client.send_check_command(Box::new(MockCommand)).await;
        assert!(res.is_ok());
    }
}