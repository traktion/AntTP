pub mod client;
pub mod config;
pub mod controller;
pub mod service;

use crate::client::cache_item::CacheItem;
use crate::controller::{
    chunk_controller, file_controller, pointer_controller, private_scratchpad_controller,
    public_archive_controller, public_scratchpad_controller, register_controller,
    graph_controller,
};
use crate::service::public_archive_service::Upload;
use ::autonomi::Client;
use actix_files::Files;
use actix_web::dev::ServerHandle;
use actix_web::web::Data;
use actix_web::{App, HttpServer, middleware::Logger, web, middleware};
use ant_evm::EvmNetwork::{ArbitrumOne, ArbitrumSepoliaTest};
use ant_evm::{EvmWallet};
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::register::{RegisterAddress, RegisterValue};
use autonomi::{BootstrapCacheConfig, ClientConfig, ClientOperatingStrategy, GraphEntry, GraphEntryAddress, InitialPeersConfig, Network, Scratchpad, ScratchpadAddress};
use awc::Client as AwcClient;
use config::anttp_config::AntTpConfig;
use log::{info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Mutex;
use foyer::{Compression, DirectFsDeviceOptions, Engine, HybridCache, HybridCacheBuilder, HybridCachePolicy, LargeEngineOptions, LfuConfig, RecoverMode, RuntimeOptions, TokioRuntimeOptions};
use tokio::task::JoinHandle;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::client::caching_client::CachingClient;

static SERVER_HANDLE: Lazy<Mutex<Option<ServerHandle>>> = Lazy::new(|| Mutex::new(None));

const API_BASE: &'static str = "/anttp-0/";

pub struct UploadState {
    upload_map: Mutex<HashMap<String, Upload>>,
}

impl UploadState {
    pub fn new() -> Self {
        UploadState {
            upload_map: Mutex::new(HashMap::<String, Upload>::new()),
        }
    }
}

pub struct UploaderState {
    uploader_map: Mutex<HashMap<String, JoinHandle<Option<ArchiveAddress>>>>,
}

impl UploaderState {
    pub fn new() -> Self {
        UploaderState {
            uploader_map: Mutex::new(HashMap::<String, JoinHandle<Option<ArchiveAddress>>>::new()),
        }
    }
}

pub struct ClientCacheState {
    register_cache: Mutex<HashMap<RegisterAddress, CacheItem<RegisterValue>>>,
    scratchpad_cache: Mutex<HashMap<ScratchpadAddress, CacheItem<Scratchpad>>>,
    graph_entry_cache: Mutex<HashMap<GraphEntryAddress, CacheItem<GraphEntry>>>,
}

impl ClientCacheState {
    pub fn new() -> Self {
        ClientCacheState {
            register_cache: Mutex::new(HashMap::new()),
            scratchpad_cache: Mutex::new(HashMap::new()),
            graph_entry_cache: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn run_server(ant_tp_config: AntTpConfig) -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(paths(
        chunk_controller::get_chunk,
        chunk_controller::get_chunk_binary,
        chunk_controller::post_chunk,
        chunk_controller::post_chunk_binary,
        pointer_controller::get_pointer,
        pointer_controller::post_pointer,
        pointer_controller::put_pointer,
        public_archive_controller::get_status_public_archive,
        public_archive_controller::post_public_archive,
        public_archive_controller::put_public_archive,
        public_scratchpad_controller::get_public_scratchpad,
        public_scratchpad_controller::post_public_scratchpad,
        public_scratchpad_controller::put_public_scratchpad,
        register_controller::get_register,
        register_controller::get_register_history,
        register_controller::post_register,
        register_controller::put_register,
        private_scratchpad_controller::get_private_scratchpad,
        private_scratchpad_controller::post_private_scratchpad,
        private_scratchpad_controller::put_private_scratchpad,
        graph_controller::get_graph_entry,
        graph_controller::post_graph_entry,
    ))]
    struct ApiDoc;

    let listen_address = ant_tp_config.listen_address.clone();
    let wallet_private_key = ant_tp_config.wallet_private_key.clone();

    // initialise safe network connection
    let evm_network = match ant_tp_config.evm_network.to_lowercase().as_str() {
        "local" => Network::new(true).unwrap(),
        "arbitrumsepoliatest" => ArbitrumSepoliaTest,
        _ => ArbitrumOne
    };
    let bootstrap_cache_config = Some(BootstrapCacheConfig::new(false).unwrap());

    let initial_peers_config = if ant_tp_config.peers.clone().is_empty() {
        InitialPeersConfig::default()
    } else {
        InitialPeersConfig {
            first: false,
            addrs: ant_tp_config.peers.clone(),
            network_contacts_url: vec![],
            local: true,
            ignore_cache: false,
            bootstrap_cache_dir: Some(bootstrap_cache_config.clone().unwrap().cache_dir),
        }
    };

    let mut strategy = ClientOperatingStrategy::default();
    strategy.chunk_cache_enabled = false; // disable cache to avoid double-caching

    let autonomi_client: Option<Client> = match Client::init_with_config(ClientConfig {
        bootstrap_cache_config: bootstrap_cache_config.clone(),
        init_peers_config: initial_peers_config,
        evm_network: evm_network.clone(),
        strategy: strategy,
        network_id: Some(1),
    }).await {
        Ok(client) => {
            Some(client)
        },
        Err(e) => {
            warn!("Failed to connect to Autonomi Network with error [{}]. Running in offline mode.", e);
            None
        },
    };

    let evm_wallet = if !wallet_private_key.is_empty() {
        EvmWallet::new_from_private_key(evm_network, wallet_private_key.as_str())
            .expect("Failed to instantiate EvmWallet.")
    } else {
        EvmWallet::new_with_random_wallet(evm_network)
    };

    let uploader_state = Data::new(UploaderState::new());
    let upload_state = Data::new(UploadState::new());
    let client_cache_state = Data::new(ClientCacheState::new());

    let hybrid_cache: HybridCache<String, Vec<u8>> = build_foyer_cache(&ant_tp_config).await;
    let hybrid_cache_data = Data::new(hybrid_cache);

    let caching_client_data = Data::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state.clone(), hybrid_cache_data.clone())
    );

    info!("Starting listener");

    let server_instance = HttpServer::new(move || {
        let logger = Logger::default();

        let mut app = App::new()
            .wrap(logger)
            .wrap(middleware::Compress::default()) // enable compression
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .route(
                format!("{}chunk/{{address}}", API_BASE).as_str(),
                web::get().to(chunk_controller::get_chunk),
            )
            .route(
                format!("{}binary/chunk/{{address}}", API_BASE).as_str(),
                web::get().to(chunk_controller::get_chunk_binary),
            )
            .route(
                format!("{}pointer/{{address}}", API_BASE).as_str(),
                web::get().to(pointer_controller::get_pointer),
            )
            .route(
                format!("{}public_archive/status/{{id}}", API_BASE).as_str(),
                web::get().to(public_archive_controller::get_status_public_archive),
            )
            .route(
                format!("{}public_scratchpad/{{address}}", API_BASE).as_str(),
                web::get().to(public_scratchpad_controller::get_public_scratchpad),
            )
            .route(
                format!("{}register/{{address}}", API_BASE).as_str(),
                web::get().to(register_controller::get_register),
            )
            .route(
                format!("{}register_history/{{address}}", API_BASE).as_str(),
                web::get().to(register_controller::get_register_history),
            )
            .route(
                format!("{}private_scratchpad/{{address}}/{{name}}", API_BASE).as_str(),
                web::get().to(private_scratchpad_controller::get_private_scratchpad),
            )
            .route(
                format!("{}graph_entry/{{address}}", API_BASE).as_str(),
                web::get().to(graph_controller::get_graph_entry)
            )
            .route(
                "/{path:.*}",
                web::get().to(file_controller::get_public_data),
            )
            .app_data(Data::new(ant_tp_config.clone()))
            .app_data(caching_client_data.clone())
            .app_data(Data::new(AwcClient::default()))
            .app_data(Data::new(evm_wallet.clone()))
            .app_data(uploader_state.clone())
            .app_data(upload_state.clone())
            .app_data(client_cache_state.clone())
            .app_data(hybrid_cache_data.clone());

        if !ant_tp_config.uploads_disabled {
            app = app
                .route(
                    format!("{}chunk", API_BASE).as_str(),
                    web::post().to(chunk_controller::post_chunk),
                )
                .route(
                    format!("{}binary/chunk", API_BASE).as_str(),
                    web::post().to(chunk_controller::post_chunk_binary),
                )
                .route(
                    format!("{}pointer", API_BASE).as_str(),
                    web::post().to(pointer_controller::post_pointer),
                )
                .route(
                    format!("{}pointer/{{address}}", API_BASE).as_str(),
                    web::put().to(pointer_controller::put_pointer),
                )
                .route(
                    format!("{}multipart/public_archive", API_BASE).as_str(),
                    web::post().to(public_archive_controller::post_public_archive),
                )
                .route(
                    format!("{}multipart/public_archive/{{address}}", API_BASE).as_str(),
                    web::put().to(public_archive_controller::put_public_archive),
                )
                .route(
                    format!("{}public_scratchpad", API_BASE).as_str(),
                    web::post().to(public_scratchpad_controller::post_public_scratchpad),
                )
                .route(
                    format!("{}public_scratchpad/{{address}}", API_BASE).as_str(),
                    web::put().to(public_scratchpad_controller::put_public_scratchpad),
                )
                .route(
                    format!("{}register", API_BASE).as_str(),
                    web::post().to(register_controller::post_register),
                )
                .route(
                    format!("{}register/{{address}}", API_BASE).as_str(),
                    web::put().to(register_controller::put_register),
                )
                .route(
                    format!("{}private_scratchpad", API_BASE).as_str(),
                    web::post().to(private_scratchpad_controller::post_private_scratchpad),
                )
                .route(
                    format!("{}private_scratchpad/{{address}}", API_BASE).as_str(),
                    web::put().to(private_scratchpad_controller::put_private_scratchpad),
                )
                .route(
                    format!("{}graph_entry", API_BASE).as_str(),
                    web::post().to(graph_controller::post_graph_entry));
        };

        if ant_tp_config.static_file_directory != "" {
            app.service(Files::new(
                "/static",
                ant_tp_config.static_file_directory.clone(),
            ))
        } else {
            app
        }
    })
    .bind(listen_address)?
    .run();

    {
        let mut guard = SERVER_HANDLE.lock().unwrap();
        *guard = Some(server_instance.handle());
    }

    server_instance.await
}

async fn build_foyer_cache(app_config: &AntTpConfig) -> HybridCache<String, Vec<u8>> {
    let cache_dir = if app_config.map_cache_directory.is_empty() {
        env::temp_dir().to_str().unwrap().to_owned() + "/anttp/cache/"
    } else {
        app_config.map_cache_directory.clone()
    };
    HybridCacheBuilder::new()
        .with_name("anttp-hybrid-cache")
        .with_flush_on_close(true)
        .with_policy(HybridCachePolicy::WriteOnInsertion)
        .memory(app_config.immutable_memory_cache_size)
        .with_shards(4)
        .with_eviction_config(LfuConfig::default())
        .storage(Engine::Large(LargeEngineOptions::default())) // use large object disk cache engine only
        .with_device_options(DirectFsDeviceOptions::new(Path::new(cache_dir.as_str()))
            .with_capacity(app_config.immutable_disk_cache_size * 1024 * 1024))
        .with_recover_mode(RecoverMode::Quiet)
        .with_compression(Compression::None) // as chunks are already compressed
        .with_runtime_options(RuntimeOptions::Separated {
            read_runtime_options: TokioRuntimeOptions {
                worker_threads: app_config.download_threads,
                max_blocking_threads: 8,
            },
            write_runtime_options: TokioRuntimeOptions {
                worker_threads: app_config.download_threads,
                max_blocking_threads: 8,
            },
        })
        .build()
        .await.unwrap()
}

pub async fn stop_server() -> Result<(), String> {
    let handle_opt = {
        let mut guard = SERVER_HANDLE.lock().unwrap();
        guard.take()
    };

    if let Some(handle) = handle_opt {
        info!("Stopping server gracefully...");
        handle.stop(true).await;
        info!("Server stopped");
        Ok(())
    } else {
        Err("Server handle not found or already stopped".to_string())
    }
}
