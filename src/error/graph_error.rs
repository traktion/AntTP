use thiserror::Error;
use serde::Serialize;
use actix_http::StatusCode;
use actix_web::HttpResponse;
use actix_web::http::header::ContentType;
use autonomi::AddressParseError;
use autonomi::client::ConnectError;
use hex::FromHexError;
use crate::error::{CreateError, GetError};

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

impl From<foyer::Error> for GraphError {
    fn from(value: foyer::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<rmp_serde::decode::Error> for GraphError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Self::GetError(value.into())
    }
}

impl From<ConnectError> for GraphError {
    fn from(value: ConnectError) -> Self {
        Self::GetError(value.into())
    }
}

impl From<FromHexError> for GraphError {
    fn from(value: FromHexError) -> Self {
        Self::GetError(value.into())
    }
}

impl From<AddressParseError> for GraphError {
    fn from(value: AddressParseError) -> Self {
        Self::GetError(value.into())
    }
}

impl actix_web::ResponseError for GraphError {
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