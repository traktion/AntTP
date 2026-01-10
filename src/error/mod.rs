use crate::client::command::Command;
use actix_http::StatusCode;
use actix_web::http::header::ContentType;
use actix_web::{error, HttpResponse};
use autonomi::client::ConnectError;
use autonomi::register::RegisterError;
use autonomi::AddressParseError;
use hex::FromHexError;
use serde::Serialize;
use std::fmt::Debug;
use std::io;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub mod chunk_error;
pub mod graph_error;
pub mod pointer_error;
pub mod public_archive_error;
pub mod public_data_error;
pub mod register_error;
pub mod scratchpad_error;
pub mod archive_error;
// todo: split into a crate + separate files

#[derive(Error, Debug, Serialize)]
pub enum CreateError {
    #[error("command creation failed: {0}")]
    Command(String),
    #[error("encryption failed: {0}")]
    Encryption(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("source data missing: {0}")]
    TemporaryStorage(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("data key missing: {0}")]
    DataKeyMissing(String),
    #[error("network is offline: {0}")]
    NetworkOffline(String),
}

impl From<SendError<Box<dyn Command>>> for CreateError {
    fn from(value: SendError<Box<dyn Command>>) -> Self {
        Self::Command(value.to_string())
    }
}

impl error::ResponseError for CreateError {
    fn status_code(&self) -> StatusCode {
        match self {
            CreateError::Command(_) => StatusCode::BAD_GATEWAY,
            CreateError::Encryption(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CreateError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CreateError::TemporaryStorage(_) => StatusCode::INSUFFICIENT_STORAGE,
            CreateError::InvalidData(_) => StatusCode::BAD_REQUEST,
            CreateError::DataKeyMissing(_) => StatusCode::PRECONDITION_FAILED,
            CreateError::NetworkOffline(_) => StatusCode::BAD_GATEWAY
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum UpdateError {
    #[error("address not derived from: {0}")]
    NotDerivedAddress(String),
    #[error("app key missing: {0}")]
    AppKeyMissing(String),
    #[error("source data missing: {0}")]
    TemporaryStorage(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("command creation failed: {0}")]
    Command(String),
    #[error("network is offline: {0}")]
    NetworkOffline(String),
}

impl From<SendError<Box<dyn Command>>> for UpdateError {
    fn from(value: SendError<Box<dyn Command>>) -> Self {
        Self::Command(value.to_string())
    }
}

impl From<io::Error> for UpdateError {
    fn from(value: io::Error) -> Self {
        Self::TemporaryStorage(value.to_string())
    }
}

impl error::ResponseError for UpdateError {
    fn status_code(&self) -> StatusCode {
        match self {
            UpdateError::NotDerivedAddress(_) => StatusCode::BAD_REQUEST,
            UpdateError::AppKeyMissing(_) => StatusCode::PRECONDITION_FAILED,
            UpdateError::Command(_) => StatusCode::BAD_GATEWAY,
            UpdateError::NetworkOffline(_) => StatusCode::BAD_GATEWAY,
            UpdateError::TemporaryStorage(_) => StatusCode::INSUFFICIENT_STORAGE,
            UpdateError::InvalidData(_) => StatusCode::BAD_REQUEST
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum GetError {
    #[error("record not found: {0}")]
    RecordNotFound(String),
    #[error("bad address: {0}")]
    BadAddress(String),
    #[error("address not derived from: {0}")]
    NotDerivedAddress(String),
    #[error("derivation name missing: {0}")]
    DerivationNameMissing(String),
    #[error("derivation key missing: {0}")]
    DerivationKeyMissing(String),
    #[error("decryption failed: {0}")]
    Decryption(String),
    #[error("command creation failed: {0}")]
    Command(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("access not allowed: {0}")]
    AccessNotAllowed(String),
    #[error("network is offline: {0}")]
    NetworkOffline(String),
}

impl error::ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetError::RecordNotFound(_) => StatusCode::NOT_FOUND,
            GetError::BadAddress(_) => StatusCode::BAD_REQUEST,
            GetError::NotDerivedAddress(_) => StatusCode::PRECONDITION_FAILED,
            GetError::DerivationNameMissing(_) => StatusCode::BAD_REQUEST,
            GetError::DerivationKeyMissing(_) => StatusCode::BAD_REQUEST,
            GetError::Decryption(_) => StatusCode::BAD_REQUEST,
            GetError::Command(_) => StatusCode::BAD_GATEWAY,
            GetError::Decode(_) => StatusCode::BAD_REQUEST,
            GetError::AccessNotAllowed(_) => StatusCode::FORBIDDEN,
            GetError::NetworkOffline(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

impl From<SendError<Box<dyn Command>>> for GetError {
    fn from(value: SendError<Box<dyn Command>>) -> Self {
        Self::Command(value.to_string())
    }
}

impl From<foyer::Error> for GetError {
    fn from(value: foyer::Error) -> Self {
        Self::RecordNotFound(value.to_string())
    }
}

impl From<rmp_serde::decode::Error> for GetError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Self::Decode(value.to_string())
    }
}

impl From<ConnectError> for GetError {
    fn from(value: ConnectError) -> Self {
        Self::NetworkOffline(value.to_string())
    }
}

impl From<AddressParseError> for GetError {
    fn from(value: AddressParseError) -> Self {
        match value {
            AddressParseError::PublicKey(_) => Self::Decryption(value.to_string()),
            _ => Self::BadAddress(value.to_string()),
        }
    }
}

impl From<FromHexError> for GetError {
    fn from(value: FromHexError) -> Self {
        Self::BadAddress(value.to_string())
    }
}

impl From<RegisterError> for GetError {
    fn from(value: RegisterError) -> Self {
        match value {
            // todo: add other error mappings
            RegisterError::Corrupt(_) => Self::Decryption(value.to_string()),
            _ => Self::BadAddress(value.to_string()),
        }
    }
}

#[derive(Error, Debug, Serialize)]
pub enum GetStreamError {
    #[error("bad range: {0}")]
    BadRange(String),
    #[error("bad receiver: {0}")]
    BadReceiver(String),
}

impl error::ResponseError for GetStreamError {}

#[derive(Error, Debug, Serialize)]
pub enum CheckError {
    #[error("record not found: {0}")]
    RecordNotFound(String),
    #[error("command creation failed: {0}")]
    Command(String),
}

impl error::ResponseError for CheckError {}

impl From<SendError<Box<dyn Command>>> for CheckError {
    fn from(value: SendError<Box<dyn Command>>) -> Self {
        Self::Command(value.to_string())
    }
}

impl From<foyer::Error> for CheckError {
    fn from(value: foyer::Error) -> Self {
        Self::RecordNotFound(value.to_string())
    }
}