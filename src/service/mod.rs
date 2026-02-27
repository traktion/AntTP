pub mod public_archive_service;
pub mod tarchive_service;
pub mod archive_service;
pub mod file_service;
pub mod register_service;
pub mod pointer_service;
pub mod resolver_service;
pub mod archive_helper;
pub mod html_directory_renderer;
pub mod scratchpad_service;
pub mod chunk_service;
pub mod graph_service;
pub mod public_data_service;
pub mod command_service;
pub mod header_builder;
pub mod access_checker;
pub mod bookmark_resolver;
pub mod pointer_name_resolver;
pub mod pnr_service;
pub mod key_value_service;

use crate::config::anttp_config::AntTpConfig;
use crate::controller::DataKey;
use crate::error::CreateError;
use autonomi::SecretKey;

pub fn get_secret_key(ant_tp_config: &AntTpConfig, data_key: DataKey) -> Result<SecretKey, CreateError> {
    match data_key {
        DataKey::Resolver => ant_tp_config.get_resolver_private_key(),
        DataKey::Personal => ant_tp_config.get_app_private_key(),
        DataKey::Custom(key) => match SecretKey::from_hex(&key.as_str()) {
            Ok(secret_key) => Ok(secret_key),
            Err(e) => Err(CreateError::DataKeyMissing(e.to_string()))
        }
    }
}