pub mod client;
pub mod config;
pub mod controller;
pub mod service;
pub mod model;
pub mod error;
pub mod tool;
pub mod grpc;

use crate::controller::{pnr_controller, chunk_controller, command_controller, connect_controller, file_controller, graph_controller, pointer_controller, private_scratchpad_controller, public_archive_controller, public_data_controller, public_scratchpad_controller, register_controller};
use actix_files::Files;
use actix_web::dev::ServerHandle;
use actix_web::web::Data;
use actix_web::{middleware, middleware::Logger, web, App, HttpServer};
use ant_evm::EvmNetwork::{ArbitrumOne, ArbitrumSepoliaTest};
use ant_evm::EvmWallet;
use autonomi::Network;
use config::anttp_config::AntTpConfig;
use log::info;
use once_cell::sync::Lazy;
use std::{env, io};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use actix_web::http::Method;
use async_job::Runner;
use foyer::{BlockEngineBuilder, Compression, DeviceBuilder, FsDeviceBuilder, HybridCache, HybridCacheBuilder, HybridCachePolicy, IoEngineBuilder, LfuConfig, PsyncIoEngineBuilder, RecoverMode, RuntimeOptions, TokioRuntimeOptions};
use indexmap::IndexMap;
use rmcp_actix_web::transport::{StreamableHttpService};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tonic::transport::Server;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::client::CachingClient;
use crate::client::client_harness::ClientHarness;
use client::command::executor::Executor;
use crate::client::command::access_checker::update_access_checker_command::UpdateAccessCheckerCommand;
use crate::client::command::bookmark_resolver::update_bookmark_resolver_command::UpdateBookmarkResolverCommand;
use crate::client::command::Command;
use crate::client::command::command_details::CommandDetails;
use crate::service::access_checker::AccessChecker;
use crate::service::bookmark_resolver::BookmarkResolver;
use crate::service::pointer_name_resolver::PointerNameResolver;
use crate::service::pnr_service::PnrService;
use crate::service::chunk_service::ChunkService;
use crate::service::command_service::CommandService;
use crate::service::file_service::FileService;
use crate::service::graph_service::GraphService;
use crate::service::pointer_service::PointerService;
use crate::service::public_archive_service::PublicArchiveService;
use crate::service::public_data_service::PublicDataService;
use crate::service::register_service::RegisterService;
use crate::service::resolver_service::ResolverService;
use crate::service::scratchpad_service::ScratchpadService;
use crate::tool::McpTool;
use crate::grpc::pointer_handler::{PointerHandler, PointerServiceServer};
use crate::grpc::register_handler::{RegisterHandler, RegisterServiceServer};
use crate::grpc::chunk_handler::{ChunkHandler, ChunkServiceServer};
use crate::grpc::graph_handler::{GraphHandler, GraphServiceServer};
use crate::grpc::command_handler::{CommandHandler, CommandServiceServer};
use crate::grpc::pnr_handler::{PnrHandler, PnrServiceServer};
use crate::grpc::public_data_handler::{PublicDataHandler, PublicServiceServer};
use crate::grpc::public_archive_handler::{PublicArchiveHandler, PublicArchiveServiceServer};

static ACTIX_SERVER_HANDLE: Lazy<Mutex<Option<ServerHandle>>> = Lazy::new(|| Mutex::new(None));
static TONIC_SERVER_HANDLE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

const API_BASE: &'static str = "/anttp-0/";

pub async fn run_server(ant_tp_config: AntTpConfig) -> io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(paths(
        chunk_controller::get_chunk,
        chunk_controller::get_chunk_binary,
        chunk_controller::post_chunk,
        chunk_controller::post_chunk_binary,
        pointer_controller::get_pointer,
        pointer_controller::post_pointer,
        pointer_controller::put_pointer,
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
        pnr_controller::post_pnr
    ))]
    struct ApiDoc;

    let listen_address = ant_tp_config.listen_address.clone();
    let https_listen_address = ant_tp_config.https_listen_address.clone();
    let grpc_listen_address = ant_tp_config.grpc_listen_address.clone();
    let wallet_private_key = ant_tp_config.wallet_private_key.clone();

    // initialise safe network connection
    let evm_network = match ant_tp_config.evm_network.to_lowercase().as_str() {
        "local" => Network::new(true).unwrap(),
        "arbitrumsepoliatest" => ArbitrumSepoliaTest,
        _ => ArbitrumOne
    };

    let client_harness_data = Data::new(Mutex::new(ClientHarness::new(evm_network.clone(), ant_tp_config.clone())));

    let evm_wallet_data = if !wallet_private_key.is_empty() {
        Data::new(EvmWallet::new_from_private_key(evm_network, wallet_private_key.as_str())
            .expect("Failed to instantiate EvmWallet."))
    } else {
        Data::new(EvmWallet::new_with_random_wallet(evm_network))
    };
    
    let hybrid_cache_data: Data<HybridCache<String, Vec<u8>>> = Data::new(build_foyer_cache(&ant_tp_config).await);

    let command_status_data = Data::new(Mutex::new(IndexMap::<u128, CommandDetails>::with_capacity(ant_tp_config.command_buffer_size * 2)));
    let command_executor = Executor::start(ant_tp_config.command_buffer_size, command_status_data.clone()).await;
    let command_executor_data = Data::new(command_executor.clone());

    let caching_client = CachingClient::new(client_harness_data, ant_tp_config.clone(), hybrid_cache_data.clone(), command_executor_data.clone());
    let caching_client_data = Data::new(caching_client.clone());

    let pointer_name_resolver_data = Data::new(PointerNameResolver::new(caching_client.clone(), ant_tp_config.get_resolver_private_key().unwrap(), ant_tp_config.cached_mutable_ttl));

    let bookmark_resolver_data = hydrate_bookmark_resolver(
        &ant_tp_config, &command_executor, &caching_client, pointer_name_resolver_data.clone()).await;
    let access_checker_data = hydrate_access_checker(
        &ant_tp_config, &command_executor, &caching_client, &bookmark_resolver_data, &pointer_name_resolver_data).await;

    let resolver_service_data = Data::new(
        ResolverService::new(caching_client_data.get_ref().clone(), access_checker_data.clone(), bookmark_resolver_data.clone(), pointer_name_resolver_data.clone(), ant_tp_config.cached_mutable_ttl)
    );

    // schedule idle disconnects for client_harness
    Runner::new().add(Box::new(caching_client_data.get_ref().clone())).run().await;

    // define services
    let public_archive_service_data = Data::new(PublicArchiveService::new(FileService::new(caching_client_data.get_ref().clone(), ant_tp_config.download_threads), caching_client.clone()));
    let command_service_data = Data::new(CommandService::new(command_status_data.clone()));
    let chunk_service_data = Data::new(ChunkService::new(caching_client_data.get_ref().clone()));
    let graph_service_data = Data::new(GraphService::new(caching_client_data.get_ref().clone(), ant_tp_config.clone()));
    let pointer_service_data = Data::new(PointerService::new(caching_client_data.get_ref().clone(), ant_tp_config.clone(), resolver_service_data.get_ref().clone()));
    let public_data_service_data = Data::new(PublicDataService::new(caching_client_data.get_ref().clone()));
    let register_service_data = Data::new(RegisterService::new(caching_client_data.get_ref().clone(), ant_tp_config.clone(), resolver_service_data.get_ref().clone()));
    let scratchpad_service_data = Data::new(ScratchpadService::new(caching_client_data.get_ref().clone(), ant_tp_config.clone()));
    let pnr_service_data = Data::new(PnrService::new(caching_client_data.get_ref().clone(), pointer_service_data.clone()));

    // MCP
    let mcp_tool = McpTool::new(
        command_service_data.clone(),
        chunk_service_data.clone(),
        pnr_service_data.clone(),
        public_data_service_data.clone(),
        pointer_service_data.clone(),
        register_service_data.clone(),
        graph_service_data.clone(),
        public_archive_service_data.clone(),
        scratchpad_service_data.clone(),
        evm_wallet_data.clone()
    );
    let mcp_tool_service = StreamableHttpService::builder()
        .service_factory(Arc::new(move || { Ok(mcp_tool.clone()) }))
        .session_manager(Arc::new(LocalSessionManager::default())) // Local session management
        .stateful_mode(true) // Enable stateful session management
        .sse_keep_alive(Duration::from_secs(30)) // Keep-alive pings every 30 seconds
        .build();

    // GRPC
    let pointer_handler = PointerHandler::new(pointer_service_data.clone(), evm_wallet_data.clone());
    let register_handler = RegisterHandler::new(register_service_data.clone(), evm_wallet_data.clone());
    let chunk_handler = ChunkHandler::new(chunk_service_data.clone(), evm_wallet_data.clone());
    let graph_handler = GraphHandler::new(graph_service_data.clone(), evm_wallet_data.clone());
    let command_handler = CommandHandler::new(command_service_data.clone());
    let pnr_handler = PnrHandler::new(pnr_service_data.clone(), evm_wallet_data.clone());
    let public_data_handler = PublicDataHandler::new(public_data_service_data.clone(), evm_wallet_data.clone());
    let public_archive_handler = PublicArchiveHandler::new(public_archive_service_data.clone(), evm_wallet_data.clone());
    let tonic_server = async move {
        tokio::task::spawn(
            Server::builder()
                .add_service(PointerServiceServer::new(pointer_handler))
                .add_service(RegisterServiceServer::new(register_handler))
                .add_service(ChunkServiceServer::new(chunk_handler))
                .add_service(GraphServiceServer::new(graph_handler))
                .add_service(CommandServiceServer::new(command_handler))
                .add_service(PnrServiceServer::new(pnr_handler))
                .add_service(PublicServiceServer::new(public_data_handler))
                .add_service(PublicArchiveServiceServer::new(public_archive_handler))
                .serve(grpc_listen_address),
        )
    };

    let actix_server = HttpServer::new(move || {
        let logger = Logger::default();

        let mut app = App::new()
            .wrap(logger)
            .wrap(middleware::Compress::default()) // enable compression
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .route(
                "",
                web::method(Method::CONNECT).to(connect_controller::forward)
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
            .app_data(evm_wallet_data.clone())
            .app_data(hybrid_cache_data.clone())
            .app_data(command_status_data.clone())
            .app_data(access_checker_data.clone())
            .app_data(bookmark_resolver_data.clone())
            .app_data(pointer_name_resolver_data.clone())
            .app_data(command_service_data.clone())
            .app_data(chunk_service_data.clone())
            .app_data(graph_service_data.clone())
            .app_data(pointer_service_data.clone())
            .app_data(public_archive_service_data.clone())
            .app_data(public_data_service_data.clone())
            .app_data(register_service_data.clone())
            .app_data(resolver_service_data.clone())
            .app_data(scratchpad_service_data.clone())
            .app_data(pnr_service_data.clone())
            .app_data(web::PayloadConfig::new(1024 * 1024 * 10));

        if !ant_tp_config.uploads_disabled {
            app = app
                .service(
                    web::scope("/mcp-0")
                        .service(mcp_tool_service.clone().scope())
                )
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
                )
                .route(
                    format!("{}pnr", API_BASE).as_str(),
                    web::post().to(pnr_controller::post_pnr)
                )
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
        .bind_rustls_0_23(https_listen_address, rustls_config())?
        .run();

    let mut guard = TONIC_SERVER_HANDLE.lock().await;
    *guard = Some("tonic_server".to_string());
    info!("Starting Tonic (gRPC) listener on port {}", grpc_listen_address);
    tonic_server.await;

    let mut guard = ACTIX_SERVER_HANDLE.lock().await;
    *guard = Some(actix_server.handle());

    info!("Starting Actix (HTTP) listener");
    actix_server.await
}

async fn hydrate_access_checker(ant_tp_config: &AntTpConfig,
                                command_executor: &Sender<Box<dyn Command>>,
                                caching_client: &CachingClient,
                                bookmark_resolver_data: &Data<Mutex<BookmarkResolver>>,
                                pointer_name_resolver_data: &Data<PointerNameResolver>,
) -> Data<Mutex<AccessChecker>> {
    let access_checker_data = Data::new(Mutex::new(AccessChecker::new()));
    let update_access_checker_command = Box::new(
        UpdateAccessCheckerCommand::new(
            Data::new(Mutex::new(caching_client.clone())),
            ant_tp_config.clone(),
            access_checker_data.clone(),
            bookmark_resolver_data.clone(),
            pointer_name_resolver_data.clone(),
        ),
    );
    command_executor.send(update_access_checker_command).await.expect("failed to send UpdateAccessCheckerCommand");
    access_checker_data
}

async fn hydrate_bookmark_resolver(ant_tp_config: &AntTpConfig,
                                   command_executor: &Sender<Box<dyn Command>>,
                                   caching_client: &CachingClient,
                                   pointer_name_resolver_data: Data<PointerNameResolver>,
) -> Data<Mutex<BookmarkResolver>> {
    let access_checker_data = Data::new(Mutex::new(AccessChecker::new()));
    let bookmark_resolver_data = Data::new(Mutex::new(BookmarkResolver::new()));
    let update_bookmark_resolver_command = Box::new(
        UpdateBookmarkResolverCommand::new(
            Data::new(Mutex::new(caching_client.clone())),
            ant_tp_config.clone(),
            access_checker_data.clone(),
            bookmark_resolver_data.clone(),
            pointer_name_resolver_data.clone(),
        )
    );
    command_executor.send(update_bookmark_resolver_command).await.expect("failed to send UpdateBookmarkResolverCommand");
    bookmark_resolver_data
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
        let mut guard = ACTIX_SERVER_HANDLE.lock().await;
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

fn rustls_config() -> rustls::ServerConfig {
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(["*".to_owned()]).unwrap();
    let cert_file = cert.pem();
    let key_file = signing_key.serialize_pem();

    let cert_file = &mut io::BufReader::new(cert_file.as_bytes());
    let key_file = &mut io::BufReader::new(key_file.as_bytes());

    let cert_chain = rustls_pemfile::certs(cert_file)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut keys = rustls_pemfile::pkcs8_private_keys(key_file)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();


    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            cert_chain,
            rustls::pki_types::PrivateKeyDer::Pkcs8(keys.remove(0)),
        )
        .unwrap();

    const H1_ALPN: &[u8] = b"http/1.1";
    const H2_ALPN: &[u8] = b"h2";

    config.alpn_protocols.push(H2_ALPN.to_vec());
    config.alpn_protocols.push(H1_ALPN.to_vec());

    config
}
