use actix_web::HttpRequest;

pub mod pointer_controller;
pub mod register_controller;
pub mod file_controller;
pub mod public_archive_controller;
pub mod tarchive_controller;
pub mod private_scratchpad_controller;
pub mod public_scratchpad_controller;
pub mod chunk_controller;
pub mod graph_controller;
pub mod public_data_controller;
pub mod command_controller;
pub mod connect_controller;
pub mod pnr_controller;

#[derive(Clone,Debug)]
pub enum StoreType {
    Memory,
    Disk,
    Network
}

impl PartialEq for StoreType {
    fn eq(&self, other: &Self) -> bool {
        match other {
            StoreType::Memory => matches!(self, StoreType::Memory),
            StoreType::Disk => matches!(self, StoreType::Disk),
            StoreType::Network => matches!(self, StoreType::Network),
        }
    }
}

impl From<String> for StoreType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "memory" => StoreType::Memory,
            "disk" => StoreType::Disk,
            _ => StoreType::Network
        }
    }
}

fn get_store_type(request: &HttpRequest) -> StoreType {
    StoreType::from(
        match request.headers().get("x-cache-only") {
            Some(x_cache_only) => x_cache_only.to_str().unwrap_or("").to_string(),
            None => "".to_string()
        }
    )
}

#[derive(Clone,Debug)]
pub enum DataKey {
    Personal,
    Resolver,
    Custom(String),
}

impl From<String> for DataKey {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "resolver" => DataKey::Resolver,
            "personal" | "" => DataKey::Personal,
            custom => DataKey::Custom(custom.to_string())
        }
    }
}

fn data_key(request: &HttpRequest) -> DataKey {
    match request.headers().get("x-data-key") {
        Some(header_value) => DataKey::from(header_value.to_str().unwrap_or("").to_string()),
        None => DataKey::Personal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_type_eq() {
        assert_eq!(StoreType::Memory == StoreType::Memory, true);
        assert_eq!(StoreType::Memory == StoreType::Disk, false);
        assert_eq!(StoreType::Memory == StoreType::Network, false);

        assert_eq!(StoreType::Disk == StoreType::Disk, true);
        assert_eq!(StoreType::Disk == StoreType::Memory, false);
        assert_eq!(StoreType::Disk == StoreType::Network, false);

        assert_eq!(StoreType::Network == StoreType::Network, true);
        assert_eq!(StoreType::Network == StoreType::Memory, false);
        assert_eq!(StoreType::Network == StoreType::Disk, false);
    }

    #[test]
    fn test_cache_type_from_string() {
        assert_eq!(StoreType::from("memory".to_string()), StoreType::Memory);
        assert_eq!(StoreType::from("disk".to_string()), StoreType::Disk);
        assert_eq!(StoreType::from("network".to_string()), StoreType::Network);
        assert_eq!(StoreType::from("other".to_string()), StoreType::Network);
        assert_eq!(StoreType::from("".to_string()), StoreType::Network);
    }
}