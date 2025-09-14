use actix_web::HttpRequest;

pub mod pointer_controller;
pub mod register_controller;
pub mod file_controller;
pub mod public_archive_controller;
pub mod private_scratchpad_controller;
pub mod public_scratchpad_controller;
pub mod chunk_controller;
pub mod graph_controller;

fn is_cache_only(request: HttpRequest) -> bool {
    request.headers()
        .get("x-cache-only")
        .is_some_and(|v| v == "true")
}