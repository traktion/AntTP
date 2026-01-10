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
    Memory,
    Disk,
    Network
}

impl PartialEq for CacheType {
    fn eq(&self, other: &Self) -> bool {
        match other {
            CacheType::Memory => matches!(self, CacheType::Memory),
            CacheType::Disk => matches!(self, CacheType::Disk),
            CacheType::Network => matches!(self, CacheType::Network),
        }
    }
}

impl From<String> for CacheType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "memory" => CacheType::Memory,
            "disk" => CacheType::Disk,
            _ => CacheType::Network
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_type_eq() {
        assert_eq!(CacheType::Memory == CacheType::Memory, true);
        assert_eq!(CacheType::Memory == CacheType::Disk, false);
        assert_eq!(CacheType::Memory == CacheType::Network, false);

        assert_eq!(CacheType::Disk == CacheType::Disk, true);
        assert_eq!(CacheType::Disk == CacheType::Memory, false);
        assert_eq!(CacheType::Disk == CacheType::Network, false);

        assert_eq!(CacheType::Network == CacheType::Network, true);
        assert_eq!(CacheType::Network == CacheType::Memory, false);
        assert_eq!(CacheType::Network == CacheType::Disk, false);
    }

    #[test]
    fn test_cache_type_from_string() {
        assert_eq!(CacheType::from("memory".to_string()), CacheType::Memory);
        assert_eq!(CacheType::from("disk".to_string()), CacheType::Disk);
        assert_eq!(CacheType::from("network".to_string()), CacheType::Network);
        assert_eq!(CacheType::from("other".to_string()), CacheType::Network);
        assert_eq!(CacheType::from("".to_string()), CacheType::Network);
    }
}