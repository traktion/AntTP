mod anttp_config;
mod caching_client;
mod app_config;
mod archive_helper;
mod xor_helper;
mod archive_service;
mod file_service;

use std::collections::HashMap;
use std::sync::Mutex;
use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer, Responder};
use actix_files::Files;
use log::info;
use ::autonomi::Client;
use actix_multipart::Multipart;
use actix_web::dev::ConnectionInfo;
use actix_web::web::Data;
use ant_evm::EvmNetwork::ArbitrumOne;
use ant_evm::EvmWallet;
use autonomi::files::archive_public::ArchiveAddress;
use awc::Client as AwcClient;
use tokio::task::JoinHandle;
use crate::caching_client::CachingClient;
use crate::anttp_config::AntTpConfig;
use crate::archive_service::ArchiveService;
use crate::file_service::FileService;
use crate::xor_helper::XorHelper;

const DEFAULT_LOGGING: &'static str = "info,anttp=info,ant_api=warn,ant_client=warn,ant_networking=off,ant_bootstrap=error";

struct AppState {
    upload_map: Mutex<HashMap::<String, JoinHandle<ArchiveAddress>>>,
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

    let upload_map = Data::new(AppState {
        upload_map: Mutex::new(HashMap::<String, JoinHandle<ArchiveAddress>>::new()),
    });

    info!("Starting listener");

    HttpServer::new(move || {
        let logger = Logger::default();

        let app = App::new()
            .wrap(logger)
            .route("/api/v1/archive", web::post().to(post_public_archive))
            .route("/api/v1/archive/status/{id}", web::get().to(get_status_public_archive))
            .route("/{path:.*}", web::get().to(get_public_data))
            .app_data(Data::new(app_config.clone()))
            .app_data(Data::new(autonomi_client.clone()))
            .app_data(Data::new(AwcClient::default()))
            .app_data(Data::new(evm_wallet.clone()))
            .app_data(upload_map.clone());
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

async fn post_public_archive(
    payload: Multipart,
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    conn: ConnectionInfo,
    app_state: Data<AppState>,
    ant_tp_config: Data<AntTpConfig>,
)
-> impl Responder {
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), conn, app_state, ant_tp_config.clone());
    let evm_wallet = evm_wallet_data.get_ref().clone();

    info!("Creating new archive from multipart POST");
    archive_service.post_data(payload, evm_wallet).await
}

async fn get_status_public_archive(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    conn: ConnectionInfo,
    app_state: Data<AppState>,
    ant_tp_config: Data<AntTpConfig>,
) -> impl Responder {
    let id = path.into_inner();
    let archive_service = build_archive_service(autonomi_client_data.get_ref().clone(), conn, app_state, ant_tp_config.clone());

    info!("Checking upload status for [{:?}]", id);
    archive_service.get_status(id).await
}

fn build_archive_service(autonomi_client: Client, conn: ConnectionInfo, app_state: Data<AppState>, ant_tp_config: Data<AntTpConfig>) -> ArchiveService {
    let caching_autonomi_client = CachingClient::new(autonomi_client.clone());
    let xor_helper = XorHelper::new();
    let file_service = FileService::new(autonomi_client.clone(), xor_helper.clone(), conn, ant_tp_config.get_ref().clone());
    ArchiveService::new(autonomi_client, caching_autonomi_client, file_service, xor_helper, app_state)
}

async fn get_public_data(
    request: HttpRequest,
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    conn: ConnectionInfo,
    app_state: Data<AppState>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let path_parts = get_path_parts(&conn.host(), &path.into_inner());
    let xor_helper = XorHelper::new();
    let (archive_addr, archive_file_name) = xor_helper.assign_path_parts(path_parts.clone());
    let ant_tp_config = ant_tp_config_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref().clone();
    let caching_autonomi_client = CachingClient::new(autonomi_client.clone());
    let (is_found, archive, is_archive, xor_addr) = xor_helper.resolve_archive_or_file(&caching_autonomi_client, &archive_addr, &archive_file_name).await;
    let file_service = FileService::new(autonomi_client.clone(), xor_helper.clone(), conn, ant_tp_config);

    if !is_archive {
        info!("Retrieving file from XOR [{:x}]", xor_addr);
        file_service.get_data(path_parts, request, xor_addr, is_found).await
    } else {
        info!("Retrieving file from archive [{:x}]", xor_addr);
        let archive_service = ArchiveService::new(autonomi_client, caching_autonomi_client.clone(), file_service, xor_helper, app_state);
        archive_service.get_data(archive, xor_addr, request, path_parts).await
    }
}

fn get_path_parts(hostname: &str, path: &str) -> Vec<String> {
    let xor_helper = XorHelper::new();
    // assert: subdomain.autonomi as acceptable format
    if hostname.ends_with(".autonomi") {
        let mut subdomain_parts = hostname.split(".")
            .map(str::to_string)
            .collect::<Vec<String>>();
        subdomain_parts.pop(); // discard 'autonomi' suffix
        let path_parts = path.split("/")
            .map(str::to_string)
            .collect::<Vec<String>>();
        subdomain_parts.append(&mut path_parts.clone());
        subdomain_parts
    } else if xor_helper.is_xor(&hostname.to_string()) {
        let mut subdomain_parts = Vec::new();
        subdomain_parts.push(hostname.to_string());
        let path_parts = path.split("/")
            .map(str::to_string)
            .collect::<Vec<String>>();
        subdomain_parts.append(&mut path_parts.clone());
        subdomain_parts
    } else {
        let path_parts = path.split("/")
            .map(str::to_string)
            .collect::<Vec<String>>();
        path_parts.clone()
    }
}