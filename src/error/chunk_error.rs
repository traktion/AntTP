use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use thiserror::Error;
use serde::Serialize;
use crate::error::{CreateError, GetError, GetStreamError};

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

impl From<foyer::Error> for ChunkError {
    fn from(value: foyer::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for ChunkError {
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