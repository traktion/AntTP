use std::fmt::Debug;
use actix_http::StatusCode;
use actix_web::{error, HttpResponse};
use actix_web::http::header::ContentType;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use crate::client::command::Command;

// todo: split into a crate + separate files

#[derive(Error, Debug, Serialize)]
pub enum ChunkError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("get error: {0}")]
    GetError(GetError),
    #[error("get stream error: {0}")]
    GetStreamError(GetStreamError),
}

impl From<CreateError> for ChunkError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<GetError> for ChunkError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl error::ResponseError for ChunkError {
    fn status_code(&self) -> StatusCode {
        match self {
            ChunkError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

impl From<GetStreamError> for ChunkError {
    fn from(value: GetStreamError) -> Self {
        Self::GetStreamError(value)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum GraphError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(String),
    #[error("get error: {0}")]
    GetError(GetError),
}

impl From<CreateError> for GraphError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<GetError> for GraphError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl error::ResponseError for GraphError {
    fn status_code(&self) -> StatusCode {
        match self {
            GraphError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum PointerError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(UpdateError),
    #[error("get error: {0}")]
    GetError(GetError),
    #[error("check error: {0}")]
    CheckError(CheckError),
}

impl From<CreateError> for PointerError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<UpdateError> for PointerError {
    fn from(value: UpdateError) -> Self {
        Self::UpdateError(value)
    }
}

impl From<GetError> for PointerError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl From<CheckError> for PointerError {
    fn from(value: CheckError) -> Self {
        Self::CheckError(value)
    }
}

impl error::ResponseError for PointerError {
    fn status_code(&self) -> StatusCode {
        match self {
            PointerError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum PublicArchiveError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("get error: {0}")]
    GetError(GetError),
}

impl From<CreateError> for PublicArchiveError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<GetError> for PublicArchiveError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl From<PublicDataError> for PublicArchiveError {
    fn from(value: PublicDataError) -> Self {
        value.into()
    }
}

impl error::ResponseError for PublicArchiveError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublicArchiveError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum PublicDataError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("get error: {0}")]
    GetError(GetError),
}

impl From<CreateError> for PublicDataError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<GetError> for PublicDataError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl error::ResponseError for PublicDataError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublicDataError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum RegisterError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(UpdateError),
    #[error("get error: {0}")]
    GetError(GetError),
}

impl From<CreateError> for RegisterError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<UpdateError> for RegisterError {
    fn from(value: UpdateError) -> Self {
        Self::UpdateError(value)
    }
}

impl From<GetError> for RegisterError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl error::ResponseError for RegisterError {
    fn status_code(&self) -> StatusCode {
        match self {
            RegisterError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum ScratchpadError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(UpdateError),
    #[error("get error: {0}")]
    GetError(GetError),
}

impl From<CreateError> for ScratchpadError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<UpdateError> for ScratchpadError {
    fn from(value: UpdateError) -> Self {
        Self::UpdateError(value)
    }
}

impl From<GetError> for ScratchpadError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl error::ResponseError for ScratchpadError {
    fn status_code(&self) -> StatusCode {
        match self {
            ScratchpadError::GetError(v) => v.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Error, Debug, Serialize)]
pub enum TarchiveError {
    #[error("get error: {0}")]
    GetError(GetError),
}

impl From<GetError> for TarchiveError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

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