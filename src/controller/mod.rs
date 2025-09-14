use actix_web::HttpRequest;

pub mod pointer_controller;
pub mod register_controller;
pub mod file_controller;
pub mod public_archive_controller;
pub mod private_scratchpad_controller;
pub mod public_scratchpad_controller;
pub mod chunk_controller;
pub mod graph_controller;
pub mod public_data_controller;

#[derive(Clone)]
pub enum CacheType {
    Memory, Disk
}

fn cache_only(request: HttpRequest) -> Option<CacheType> {
    match request.headers().get("x-cache-only") {
        Some(header_value) => match header_value.to_str() {
            Ok(value) => match value {
                "memory" => Some(CacheType::Memory),
                "disk" => Some(CacheType::Disk),
                _ => None
            },
            Err(_) => None
        },
        None => None,
    }
}