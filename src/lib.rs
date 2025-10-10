pub mod client;
pub mod config;
pub mod controller;
pub mod service;
pub mod model;

use crate::controller::{chunk_controller, command_controller, file_controller, graph_controller, pointer_controller, private_scratchpad_controller, public_archive_controller, public_data_controller, public_scratchpad_controller, register_controller};
use crate::service::public_archive_service::Upload;
use actix_files::Files;
use actix_web::dev::ServerHandle;
use actix_web::web::Data;
use actix_web::{middleware, middleware::Logger, web, App, HttpServer};
use ant_evm::EvmNetwork::{ArbitrumOne, ArbitrumSepoliaTest};
use ant_evm::EvmWallet;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::Network;
use config::anttp_config::AntTpConfig;
use log::info;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use async_job::Runner;
use foyer::{BlockEngineBuilder, Compression, DeviceBuilder, FsDeviceBuilder, HybridCache, HybridCacheBuilder, HybridCachePolicy, IoEngineBuilder, LfuConfig, PsyncIoEngineBuilder, RecoverMode, RuntimeOptions, TokioRuntimeOptions};
use indexmap::IndexMap;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::client::CachingClient;
use crate::client::client_harness::ClientHarness;
use client::command::executor::Executor;
use crate::client::command::command_details::CommandDetails;

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
        public_data_controller::get_public_data,
        public_data_controller::post_public_data,
        command_controller::get_commands,
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

    let client_harness_data = Data::new(Mutex::new(ClientHarness::new(evm_network.clone(), ant_tp_config.clone())));

    let evm_wallet = if !wallet_private_key.is_empty() {
        EvmWallet::new_from_private_key(evm_network, wallet_private_key.as_str())
            .expect("Failed to instantiate EvmWallet.")
    } else {
        EvmWallet::new_with_random_wallet(evm_network)
    };

    let uploader_state = Data::new(UploaderState::new());
    let upload_state = Data::new(UploadState::new());

    let hybrid_cache: HybridCache<String, Vec<u8>> = build_foyer_cache(&ant_tp_config).await;
    let hybrid_cache_data = Data::new(hybrid_cache);

    let command_status = Data::new(Mutex::new(IndexMap::<u128, CommandDetails>::with_capacity(ant_tp_config.command_buffer_size * 2)));
    let command_executor = Executor::start(ant_tp_config.command_buffer_size, command_status.clone()).await;
    let command_executor_data = Data::new(command_executor);
    
    let caching_client_data = Data::new(
        CachingClient::new(client_harness_data, ant_tp_config.clone(), hybrid_cache_data.clone(), command_executor_data.clone())
    );

    // schedule idle disconnects for client_harness
    Runner::new().add(Box::new(caching_client_data.get_ref().clone())).run().await;


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
                format!("{}binary/public_data/{{address}}", API_BASE).as_str(),
                web::get().to(public_data_controller::get_public_data)
            )
            .route(
                format!("{}command", API_BASE).as_str(),
                web::get().to(command_controller::get_commands)
            )
            .route(
                "/{path:.*}",
                web::get().to(file_controller::get_public_data),
            )
            .app_data(Data::new(ant_tp_config.clone()))
            .app_data(caching_client_data.clone())
            .app_data(Data::new(evm_wallet.clone()))
            .app_data(uploader_state.clone())
            .app_data(upload_state.clone())
            .app_data(hybrid_cache_data.clone())
            .app_data(command_status.clone());

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
                    format!("{}public_scratchpad/{{name}}", API_BASE).as_str(),
                    web::post().to(public_scratchpad_controller::post_public_scratchpad),
                )
                .route(
                    format!("{}public_scratchpad/{{address}}/{{name}}", API_BASE).as_str(),
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
                    format!("{}private_scratchpad/{{name}}", API_BASE).as_str(),
                    web::post().to(private_scratchpad_controller::post_private_scratchpad),
                )
                .route(
                    format!("{}private_scratchpad/{{address}}/{{name}}", API_BASE).as_str(),
                    web::put().to(private_scratchpad_controller::put_private_scratchpad),
                )
                .route(
                    format!("{}graph_entry", API_BASE).as_str(),
                    web::post().to(graph_controller::post_graph_entry)
                )
                .route(
                    format!("{}binary/public_data", API_BASE).as_str(),
                    web::post().to(public_data_controller::post_public_data)
                );
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
        let mut guard = SERVER_HANDLE.lock().await;
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

    let memory_cache_size = if app_config.immutable_memory_cache_size > 0 { app_config.immutable_memory_cache_size } else { 1 };
    let builder = HybridCacheBuilder::new()
        .with_name("anttp-hybrid-cache")
        .with_flush_on_close(true)
        .with_policy(HybridCachePolicy::WriteOnInsertion)
        .memory(memory_cache_size)
        .with_shards(app_config.download_threads)
        .with_eviction_config(LfuConfig::default())
        .storage();

    if app_config.immutable_disk_cache_size > 0 {
        let device = FsDeviceBuilder::new(Path::new(cache_dir.as_str()))
            .with_capacity(app_config.immutable_disk_cache_size * 1024 * 1024)
            .build().expect("Failed to build FsDevice");
        let io_engine = PsyncIoEngineBuilder::new()
            .build().await.expect("Failed to build IoEngine");

        builder
            .with_io_engine(io_engine)
            .with_engine_config(BlockEngineBuilder::new(device))
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
            }).build().await.expect("Failed to build hybrid in-memory/on-disk cache")
    } else {
        builder.build().await.expect("Failed to build in-memory cache")
    }
}

pub async fn stop_server() -> Result<(), String> {
    let handle_opt = {
        let mut guard = SERVER_HANDLE.lock().await;
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
