use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use crate::error::{CreateError, GetError};
use crate::error::public_data_error::PublicDataError;

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