use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use autonomi::client::ConnectError;
use crate::error::{CreateError, GetError, UpdateError};
use crate::error::public_data_error::PublicDataError;

#[derive(Error, Debug, Serialize)]
pub enum PublicArchiveError {
    #[error("create error: {0}")]
    CreateError(CreateError),
    #[error("update error: {0}")]
    UpdateError(UpdateError),
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

impl From<UpdateError> for PublicArchiveError {
    fn from(value: UpdateError) -> Self {
        Self::UpdateError(value)
    }
}

impl From<PublicDataError> for PublicArchiveError {
    fn from(value: PublicDataError) -> Self {
        value.into()
    }
}

impl From<ConnectError> for PublicArchiveError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for PublicArchiveError {
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