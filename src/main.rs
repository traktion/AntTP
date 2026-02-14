use anttp::config::anttp_config::AntTpConfig;

const DEFAULT_LOGGING: &'static str = "info,anttp=debug,ant_api=warn,ant_client=warn,autonomi::networking=error,ant_bootstrap=error,chunk_streamer=info";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // init logging from RUST_LOG env var with info as default
    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or(DEFAULT_LOGGING))
        .format_timestamp_millis()
        .init();
    let app_config = AntTpConfig::read_args();

    anttp::run_server(app_config).await
}