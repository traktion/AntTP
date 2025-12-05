use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use autonomi::client::ConnectError;
use thiserror::Error;
use serde::Serialize;
use crate::error::{CreateError, GetError, UpdateError};

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

impl From<foyer::Error> for ScratchpadError {
    fn from(value: foyer::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<rmp_serde::decode::Error> for ScratchpadError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<ConnectError> for ScratchpadError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for ScratchpadError {
    fn status_code(&self) -> StatusCode {
        match self {
            ScratchpadError::GetError(v) => v.status_code(),
            ScratchpadError::CreateError(v) => v.status_code(),
            ScratchpadError::UpdateError(v) => v.status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}