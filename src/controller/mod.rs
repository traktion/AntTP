use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound, ErrorPreconditionFailed};
use actix_web::{Error, HttpRequest};
use log::error;
use crate::client::error::{GetError, ScratchpadError};

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

fn handle_scratchpad_error(scratchpad_error: ScratchpadError) -> Error {
    match scratchpad_error {
        ScratchpadError::GetError(get_error) => handle_get_error(get_error),
        _ => {
            error!("internal error: {}", scratchpad_error.to_string());
            ErrorInternalServerError(scratchpad_error)
        }
    }
}

fn handle_get_error(get_error: GetError) -> Error {
    match get_error {
        GetError::RecordNotFound(message) => ErrorNotFound(message),
        GetError::BadAddress(message) => ErrorBadRequest(message),
        GetError::NotDerivedAddress(message) => ErrorPreconditionFailed(message),
        GetError::DerivationNameMissing(message) => ErrorBadRequest(message),
        _ => {
            error!("internal error: {}", get_error.to_string());
            ErrorInternalServerError(get_error)
        }
    }        
}