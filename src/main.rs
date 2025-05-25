mod controller;
mod service;
mod config;
mod client;

use std::collections::HashMap;
use std::sync::Mutex;
use actix_web::{middleware::Logger, web, App, HttpServer};
use actix_files::Files;
use log::info;
use ::autonomi::Client;
use actix_web::web::Data;
use ant_evm::EvmNetwork::ArbitrumOne;
use ant_evm::EvmWallet;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::{Pointer, PointerAddress};
use autonomi::register::{RegisterAddress, RegisterValue};
use awc::Client as AwcClient;
use tokio::task::JoinHandle;
use config::anttp_config::AntTpConfig;
use crate::client::cache_item::CacheItem;
use crate::controller::file_controller::get_public_data;
use crate::controller::pointer_controller::{get_pointer, post_pointer, put_pointer};
use crate::controller::public_archive_controller::{get_status_public_archive, post_public_archive};
use crate::controller::register_controller::{get_register, get_register_history, post_register, put_register};

const DEFAULT_LOGGING: &'static str = "info,anttp=info,ant_api=warn,ant_client=warn,ant_networking=off,ant_bootstrap=error,chunk_streamer=error";

struct UploaderState {
    upload_map: Mutex<HashMap::<String, JoinHandle<ArchiveAddress>>>
}

impl UploaderState {
    pub fn new() -> Self {
        UploaderState { 
            upload_map: Mutex::new(HashMap::<String, JoinHandle<ArchiveAddress>>::new()),
        }
    }
}

struct ClientCacheState {
    pointer_cache: Mutex<HashMap<PointerAddress, CacheItem<Pointer>>>,
    register_cache: Mutex<HashMap<RegisterAddress, CacheItem<RegisterValue>>>,
}

impl ClientCacheState {
    pub fn new() -> Self {
        ClientCacheState {
            pointer_cache: Mutex::new(HashMap::new()),
            register_cache: Mutex::new(HashMap::new())
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // init logging from RUST_LOG env var with info as default
    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or(DEFAULT_LOGGING))
        .init();

    let app_config = AntTpConfig::read_args();
    let listen_address = app_config.listen_address;
    let wallet_private_key = app_config.wallet_private_key.clone();

    // initialise safe network connection and files api
    let autonomi_client = Client::init().await.expect("Failed to connect to Autonomi Network");
    let evm_wallet = if !wallet_private_key.is_empty() {
        EvmWallet::new_from_private_key(ArbitrumOne, wallet_private_key.as_str()).expect("Failed to instantiate EvmWallet.")
    } else {
        EvmWallet::new_with_random_wallet(ArbitrumOne)
    };

    let uploader_state = Data::new(UploaderState::new());
    let client_cache_state = Data::new(ClientCacheState::new());

    info!("Starting listener");

    HttpServer::new(move || {
        let logger = Logger::default();

        let mut app = App::new()
            .wrap(logger)
            .route("/api/v1/public_archive/status/{id}", web::get().to(get_status_public_archive))
            .route("/api/v1/register/{address}", web::get().to(get_register))
            .route("/api/v1/register_history/{address}", web::get().to(get_register_history))
            .route("/api/v1/pointer/{address}", web::get().to(get_pointer))
            .route("/{path:.*}", web::get().to(get_public_data))
            .app_data(Data::new(app_config.clone()))
            .app_data(Data::new(autonomi_client.clone()))
            .app_data(Data::new(AwcClient::default()))
            .app_data(Data::new(evm_wallet.clone()))
            .app_data(uploader_state.clone())
            .app_data(client_cache_state.clone());
        if !app_config.uploads_disabled {
            app = app
                .route("/api/v1/public_archive", web::post().to(post_public_archive))
                .route("/api/v1/register", web::post().to(post_register))
                .route("/api/v1/register/{address}", web::put().to(put_register))
                .route("/api/v1/pointer", web::post().to(post_pointer))
                .route("/api/v1/pointer/{address}", web::put().to(put_pointer));
        };
        if app_config.static_file_directory != "" {
            app.service(Files::new("/static", app_config.static_file_directory.clone()))
        } else {
            app
        }
    })
        .bind(listen_address)?
        .run()
        .await
}
