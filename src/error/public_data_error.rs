use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use autonomi::client::ConnectError;
use crate::error::{CreateError, GetError};

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

impl From<ConnectError> for PublicDataError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for PublicDataError {
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