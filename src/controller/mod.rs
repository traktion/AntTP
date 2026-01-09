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
pub mod connect_controller;
pub mod pnr_controller;

#[derive(Clone,Debug)]
pub enum CacheType {
    Memory, Disk
}

fn cache_only(request: &HttpRequest) -> Option<CacheType> {
    match request.headers().get("x-cache-only")?.to_str().unwrap_or("").to_lowercase().as_str() {
        "memory" => Some(CacheType::Memory),
        "disk" => Some(CacheType::Disk),
        _ => None
    }
}

#[derive(Clone,Debug)]
pub enum DataKey {
    Personal,
    Resolver,
    Custom(String),
}

fn data_key(request: &HttpRequest) -> DataKey {
    match request.headers().get("x-data-key") {
        Some(header_value) => match header_value.to_str().unwrap_or("").to_lowercase().as_str() {
            "resolver" => DataKey::Resolver,
            "personal" | "" => DataKey::Personal,
            custom => DataKey::Custom(custom.to_string())
        }
        None => DataKey::Personal,
    }
}