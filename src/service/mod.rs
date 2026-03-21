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
pub mod crypto_service;

use crate::config::anttp_config::AntTpConfig;
use crate::controller::DataKey;
use crate::error::CreateError;
use crate::error::pointer_error::PointerError;
use crate::model::pnr::PnrRecord;
use ant_protocol::storage::ChunkAddress;
use autonomi::SecretKey;
use std::collections::HashMap;

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

pub fn validate_immutable_address(address: &str) -> Result<(), PointerError> {
    if address.len() != 64 || ChunkAddress::from_hex(address).is_err() {
        return Err(PointerError::CreateError(CreateError::InvalidData(format!(
            "Invalid immutable address: address must be a 64-character hex string, got '{}'",
            address
        ))));
    }
    Ok(())
}

pub fn validate_immutable_addresses(records: &HashMap<String, PnrRecord>) -> Result<(), PointerError> {
    for (key, record) in records {
        if let Err(e) = validate_immutable_address(&record.address) {
            return Err(PointerError::CreateError(CreateError::InvalidData(format!(
                "Invalid immutable address for record '{}': {}",
                key, e
            ))));
        }
    }
    Ok(())
}