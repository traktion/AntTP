use actix_web::{web, HttpResponse};
use actix_web::web::Data;
use log::debug;
use crate::service::resolver_service::ResolverService;

#[utoipa::path(
    get,
    path = "/anttp-0/resolve/{name}",
    params(
        ("name" = String, Path, description = "Source name or address to resolve"),
    ),
    responses(
        (status = OK, description = "Address resolved successfully", body = String),
        (status = NOT_FOUND, description = "Address could not be found")
    )
)]
pub async fn resolve(
    path: web::Path<String>,
    resolver_service: Data<ResolverService>,
) -> HttpResponse {
    let name = path.into_inner();
    debug!("Resolving name [{}]", name);

    match resolver_service.resolve_name(&name).await {
        Some(resolved_address) => HttpResponse::Ok().json(resolved_address),
        None => HttpResponse::NotFound().finish(),
    }
}
