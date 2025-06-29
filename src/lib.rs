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
use actix_web::{App, HttpServer, middleware::Logger, web};
use ant_evm::EvmNetwork::ArbitrumOne;
use ant_evm::{EvmWallet};
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::register::{RegisterAddress, RegisterValue};
use autonomi::{Chunk, ChunkAddress, GraphEntry, GraphEntryAddress, Pointer, PointerAddress, Scratchpad, ScratchpadAddress};
use awc::Client as AwcClient;
use config::anttp_config::AntTpConfig;
use log::info;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::task::JoinHandle;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

static SERVER_HANDLE: Lazy<Mutex<Option<ServerHandle>>> = Lazy::new(|| Mutex::new(None));

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
    pointer_cache: Mutex<HashMap<PointerAddress, CacheItem<Pointer>>>,
    register_cache: Mutex<HashMap<RegisterAddress, CacheItem<RegisterValue>>>,
    scratchpad_cache: Mutex<HashMap<ScratchpadAddress, CacheItem<Scratchpad>>>,
    chunk_cache: Mutex<HashMap<ChunkAddress, CacheItem<Chunk>>>,
    graph_entry_cache: Mutex<HashMap<GraphEntryAddress, CacheItem<GraphEntry>>>,
}

impl ClientCacheState {
    pub fn new() -> Self {
        ClientCacheState {
            pointer_cache: Mutex::new(HashMap::new()),
            register_cache: Mutex::new(HashMap::new()),
            scratchpad_cache: Mutex::new(HashMap::new()),
            chunk_cache: Mutex::new(HashMap::new()),
            graph_entry_cache: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn run_server(app_config: AntTpConfig) -> std::io::Result<()> {
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

    let listen_address = app_config.listen_address.clone();
    let wallet_private_key = app_config.wallet_private_key.clone();

    // initialise safe network connection
    let autonomi_client = if app_config.peers.clone().is_empty() {
        Client::init()
            .await
            .expect("Failed to connect to Autonomi Network.") 
    } else {
        Client::init_with_peers(app_config.peers.clone())
            .await
            .expect("Failed to connect to Autonomi Network.")
    };

    let evm_wallet = if !wallet_private_key.is_empty() {
        EvmWallet::new_from_private_key(ArbitrumOne, wallet_private_key.as_str())
            .expect("Failed to instantiate EvmWallet.")
    } else {
        EvmWallet::new_with_random_wallet(ArbitrumOne)
    };

    let uploader_state = Data::new(UploaderState::new());
    let upload_state = Data::new(UploadState::new());
    let client_cache_state = Data::new(ClientCacheState::new());

    info!("Starting listener");

    let server_instance = HttpServer::new(move || {
        let logger = Logger::default();

        let mut app = App::new()
            .wrap(logger)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .route(
                "/api/v1/chunk/{address}",
                web::get().to(chunk_controller::get_chunk),
            )
            .route(
                "/api/v1/binary/chunk/{address}",
                web::get().to(chunk_controller::get_chunk_binary),
            )
            .route(
                "/api/v1/pointer/{address}",
                web::get().to(pointer_controller::get_pointer),
            )
            .route(
                "/api/v1/public_archive/status/{id}",
                web::get().to(public_archive_controller::get_status_public_archive),
            )
            .route(
                "/api/v1/public_scratchpad/{address}",
                web::get().to(public_scratchpad_controller::get_public_scratchpad),
            )
            .route(
                "/api/v1/register/{address}",
                web::get().to(register_controller::get_register),
            )
            .route(
                "/api/v1/register_history/{address}",
                web::get().to(register_controller::get_register_history),
            )
            .route(
                "/api/v1/private_scratchpad/{address}/{name}",
                web::get().to(private_scratchpad_controller::get_private_scratchpad),
            )
            .route("/api/v1/graph_entry/{address}", web::get().to(graph_controller::get_graph_entry))
            .route(
                "/{path:.*}",
                web::get().to(file_controller::get_public_data),
            )
            .app_data(Data::new(app_config.clone()))
            .app_data(Data::new(autonomi_client.clone()))
            .app_data(Data::new(AwcClient::default()))
            .app_data(Data::new(evm_wallet.clone()))
            .app_data(uploader_state.clone())
            .app_data(upload_state.clone())
            .app_data(client_cache_state.clone());

        if !app_config.uploads_disabled {
            app = app
                .route(
                    "/api/v1/chunk",
                    web::post().to(chunk_controller::post_chunk),
                )
                .route(
                    "/api/v1/binary/chunk",
                    web::post().to(chunk_controller::post_chunk_binary),
                )
                .route(
                    "/api/v1/pointer",
                    web::post().to(pointer_controller::post_pointer),
                )
                .route(
                    "/api/v1/pointer/{address}",
                    web::put().to(pointer_controller::put_pointer),
                )
                .route(
                    "/api/v1/multipart/public_archive",
                    web::post().to(public_archive_controller::post_public_archive),
                )
                .route(
                    "/api/v1/multipart/public_archive/{address}",
                    web::put().to(public_archive_controller::put_public_archive),
                )
                .route(
                    "/api/v1/public_scratchpad",
                    web::post().to(public_scratchpad_controller::post_public_scratchpad),
                )
                .route(
                    "/api/v1/public_scratchpad/{address}",
                    web::put().to(public_scratchpad_controller::put_public_scratchpad),
                )
                .route(
                    "/api/v1/register",
                    web::post().to(register_controller::post_register),
                )
                .route(
                    "/api/v1/register/{address}",
                    web::put().to(register_controller::put_register),
                )
                .route(
                    "/api/v1/private_scratchpad",
                    web::post().to(private_scratchpad_controller::post_private_scratchpad),
                )
                .route(
                    "/api/v1/private_scratchpad/{address}",
                    web::put().to(private_scratchpad_controller::put_private_scratchpad),
                )
                .route("/api/v1/graph_entry", web::post().to(graph_controller::post_graph_entry));
        };

        if app_config.static_file_directory != "" {
            app.service(Files::new(
                "/static",
                app_config.static_file_directory.clone(),
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
