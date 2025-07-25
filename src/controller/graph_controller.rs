use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use autonomi::Client;
use foyer::HybridCache;
use log::info;
use crate::client::caching_client::CachingClient;
use crate::ClientCacheState;
use crate::config::anttp_config::AntTpConfig;
use crate::service::graph_service::{GraphEntry, GraphService};

#[utoipa::path(
    post,
    path = "/anttp-0/graph_entry",
    request_body(
        content = GraphEntry
    ),
    responses(
        (status = CREATED, description = "Graph entry created successfully", body = GraphEntry)
    ),
)]
pub async fn post_graph_entry(
    autonomi_client_data: Data<Client>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    graph_entry: web::Json<GraphEntry>,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let evm_wallet = evm_wallet_data.get_ref().clone();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let graph_service = GraphService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Creating new graph entry");
    graph_service.create_graph_entry(graph_entry.into_inner(), evm_wallet).await
}

#[utoipa::path(
    get,
    path = "/anttp-0/graph_entry/{address}",
    responses(
        (status = OK, description = "Graph entry found successfully", body = GraphEntry),
        (status = NOT_FOUND, description = "Graph entry was not found")
    ),
    params(
        ("address" = String, Path, description = "Graph entry address"),
    )
)]
pub async fn get_graph_entry(
    path: web::Path<String>,
    autonomi_client_data: Data<Client>,
    ant_tp_config_data: Data<AntTpConfig>,
    client_cache_state: Data<ClientCacheState>,
    hybrid_cache_data: Data<HybridCache<String, Vec<u8>>>,
) -> impl Responder {
    let address = path.into_inner();

    let autonomi_client = autonomi_client_data.get_ref();
    let ant_tp_config = ant_tp_config_data.get_ref();
    let graph_service = GraphService::new(
        CachingClient::new(autonomi_client.clone(), ant_tp_config.clone(), client_cache_state, hybrid_cache_data),
        ant_tp_config_data.get_ref().clone(),
    );

    info!("Getting graph entry at [{}]", address);
    graph_service.get_graph_entry(address).await
}