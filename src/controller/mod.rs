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
pub mod command_controller;

#[derive(Clone,Debug)]
pub enum CacheType {
    Memory, Disk
}

fn cache_only(request: HttpRequest) -> Option<CacheType> {
    match request.headers().get("x-cache-only")?.to_str().unwrap_or("") {
        "memory" => Some(CacheType::Memory),
        "disk" => Some(CacheType::Disk),
        _ => None
    }
}