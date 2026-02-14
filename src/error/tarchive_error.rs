use std::io;
use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use autonomi::AddressParseError;
use autonomi::client::ConnectError;
use crate::error::{CreateError, GetError, UpdateError};
use crate::error::public_data_error::PublicDataError;
use crate::error::chunk_error::ChunkError;

#[derive(Error, Debug, Serialize)]
pub enum TarchiveError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(UpdateError),
    #[error("get error: {0}")]
    GetError(GetError),
    #[error("chunk error: {0}")]
    ChunkError(ChunkError),
}

impl From<CreateError> for TarchiveError {
    fn from(value: CreateError) -> Self {
        Self::CreateError(value)
    }
}

impl From<GetError> for TarchiveError {
    fn from(value: GetError) -> Self {
        Self::GetError(value)
    }
}

impl From<UpdateError> for TarchiveError {
    fn from(value: UpdateError) -> Self {
        Self::UpdateError(value)
    }
}

impl From<ChunkError> for TarchiveError {
    fn from(value: ChunkError) -> Self {
        Self::ChunkError(value)
    }
}

impl From<PublicDataError> for TarchiveError {
    fn from(value: PublicDataError) -> Self {
        match value {
            PublicDataError::CreateError(e) => Self::CreateError(e),
            PublicDataError::GetError(e) => Self::GetError(e),
        }
    }
}

impl From<ConnectError> for TarchiveError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl From<io::Error> for TarchiveError {
    fn from(value: io::Error) -> Self {
        Self::UpdateError(value.into())
    }
}

impl From<AddressParseError> for TarchiveError {
    fn from(value: AddressParseError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for TarchiveError {
    fn status_code(&self) -> StatusCode {
        match self {
            TarchiveError::GetError(v) => v.status_code(),
            TarchiveError::CreateError(v) => v.status_code(),
            TarchiveError::UpdateError(v) => v.status_code(),
            TarchiveError::ChunkError(v) => v.status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}
