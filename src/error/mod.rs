use std::fmt::Debug;
use actix_http::StatusCode;
use actix_web::{error, HttpResponse};
use actix_web::http::header::ContentType;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use crate::client::command::Command;

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
    #[error("network is offline: {0}")]
    NetworkOffline(String),
}

impl From<SendError<Box<dyn Command>>> for CreateError {
    fn from(value: SendError<Box<dyn Command>>) -> Self {
        Self::Command(value.to_string())
    }
}

impl error::ResponseError for CreateError {}

#[derive(Error, Debug, Serialize)]
pub enum UpdateError {
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

impl error::ResponseError for UpdateError {}

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
            GetError::NetworkOffline(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
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