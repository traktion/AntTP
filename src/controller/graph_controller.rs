use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Data;
use ant_evm::EvmWallet;
use log::debug;
use crate::error::graph_error::GraphError;
use crate::controller::get_store_type;
use crate::service::graph_service::{GraphEntry, GraphService};

#[utoipa::path(
    post,
    path = "/anttp-0/graph_entry",
    request_body(
        content = GraphEntry
    ),
    responses(
        (status = CREATED, description = "Graph entry created successfully", body = GraphEntry),
        (status = BAD_REQUEST, description = "Graph entry body was invalid")
    ),
    params(
        ("x-store-type", Header, description = "Only persist to cache and do not publish (memory|disk|none)",
        example = "memory"),
    ),
)]
pub async fn post_graph_entry(
    graph_service: Data<GraphService>,
    evm_wallet_data: Data<EvmWallet>,
    graph_entry: web::Json<GraphEntry>,
    request: HttpRequest,
) -> Result<HttpResponse, GraphError> {
    debug!("Creating new graph entry");
    Ok(HttpResponse::Created().json(
        graph_service.create_graph_entry(graph_entry.into_inner(), evm_wallet_data.get_ref().clone(), get_store_type(&request)).await?
    ))
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
    graph_service: Data<GraphService>,
) -> Result<HttpResponse, GraphError> {
    let address = path.into_inner();
    debug!("Getting graph entry at [{}]", address);
    Ok(HttpResponse::Ok().json(graph_service.get_graph_entry(address).await?))
}
