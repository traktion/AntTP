use autonomi::client::ConnectError;
use autonomi::AddressParseError;
use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use crate::error::{CreateError, GetError, UpdateError};
use crate::error::public_archive_error::PublicArchiveError;
use crate::error::tarchive_error::TarchiveError;

#[derive(Error, Debug, Serialize)]
pub enum ArchiveError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(UpdateError),
    #[error("get error: {0}")]
    GetError(GetError),
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

impl From<AddressParseError> for ArchiveError {
    fn from(value: AddressParseError) -> Self {
        Self::GetError(value.into())
    }
}

impl From<PublicArchiveError> for ArchiveError {
    fn from(value: PublicArchiveError) -> Self {
        match value {
            PublicArchiveError::CreateError(e) => Self::CreateError(e),
            PublicArchiveError::UpdateError(e) => Self::UpdateError(e),
            PublicArchiveError::GetError(e) => Self::GetError(e),
        }
    }
}

impl From<TarchiveError> for ArchiveError {
    fn from(value: TarchiveError) -> Self {
        match value {
            TarchiveError::CreateError(e) => Self::CreateError(e),
            TarchiveError::UpdateError(e) => Self::UpdateError(e),
            TarchiveError::GetError(e) => Self::GetError(e),
            TarchiveError::ChunkError(e) => Self::GetError(GetError::Decode(e.to_string())), 
        }
    }
}

impl From<rmp_serde::decode::Error> for ArchiveError {
    fn from(value: rmp_serde::decode::Error) -> Self {
         Self::GetError(value.into())
    }
}

impl From<foyer::Error> for ArchiveError {
    fn from(value: foyer::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<ConnectError> for ArchiveError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for ArchiveError {
    fn status_code(&self) -> StatusCode {
        match self {
            ArchiveError::GetError(v) => v.status_code(),
            ArchiveError::CreateError(v) => v.status_code(),
            ArchiveError::UpdateError(v) => v.status_code(),
            ArchiveError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}