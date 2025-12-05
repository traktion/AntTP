use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use autonomi::AddressParseError;
use autonomi::client::ConnectError;
use thiserror::Error;
use serde::Serialize;
use crate::error::{CheckError, CreateError, GetError, UpdateError};

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

impl From<foyer::Error> for PointerError {
    fn from(value: foyer::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<ConnectError> for PointerError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl From<rmp_serde::decode::Error> for PointerError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<AddressParseError> for PointerError {
    fn from(value: AddressParseError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for PointerError {
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