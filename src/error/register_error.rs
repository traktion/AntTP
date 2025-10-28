use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use crate::error::{CreateError, GetError, UpdateError};

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

impl From<foyer::Error> for RegisterError {
    fn from(value: foyer::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<rmp_serde::decode::Error> for RegisterError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for RegisterError {
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