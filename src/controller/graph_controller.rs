use actix_web::{web, Responder};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::info;
use crate::client::CachingClient;
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
    caching_client_data: Data<CachingClient>,
    evm_wallet_data: Data<EvmWallet>,
    ant_tp_config_data: Data<AntTpConfig>,
    graph_entry: web::Json<GraphEntry>,
) -> impl Responder {
    let graph_service = GraphService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone()
    );

    info!("Creating new graph entry");
    graph_service.create_graph_entry(graph_entry.into_inner(), evm_wallet_data.get_ref().clone()).await
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
    caching_client_data: Data<CachingClient>,
    ant_tp_config_data: Data<AntTpConfig>,
) -> impl Responder {
    let address = path.into_inner();
    
    let graph_service = GraphService::new(
        caching_client_data.get_ref().clone(),
        ant_tp_config_data.get_ref().clone()
    );

    info!("Getting graph entry at [{}]", address);
    graph_service.get_graph_entry(address).await
}