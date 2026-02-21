use actix_web::{web, HttpResponse};
use actix_web::web::Data;
use log::debug;
use crate::model::resolve::Resolve;
use crate::service::resolver_service::ResolverService;

#[utoipa::path(
    get,
    path = "/anttp-0/resolve/{name}",
    params(
        ("name" = String, Path, description = "Source name or address"),
    ),
    responses(
        (status = OK, description = "Address resolved successfully", body = Resolve),
        (status = NOT_FOUND, description = "Address could not be resolved")
    ),
)]
pub async fn resolve(
    path: web::Path<String>,
    resolver_service: Data<ResolverService>,
) -> HttpResponse {
    let name = path.into_inner();

    debug!("Resolving address for [{}]", name);
    match resolver_service.resolve_name_item(&name).await {
        Some(resolve) => HttpResponse::Ok().json(resolve),
        None => HttpResponse::NotFound().finish(),
    }
}
