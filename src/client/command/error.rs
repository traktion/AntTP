use std::fmt::Debug;
use thiserror::Error;
use tonic::ConnectError;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("unrecoverable error: {0}")]
    Unrecoverable(String),
    #[error("recoverable error: {0}")]
    Recoverable(String),
}

impl From<ConnectError> for CommandError {
    fn from(value: ConnectError) -> Self {
        Self::Recoverable(value.to_string())
    }
}

impl From<ant_core::data::error::Error> for CommandError {
    fn from(value: ant_core::data::error::Error) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<rmp_serde::encode::Error> for CommandError {
    fn from(value: rmp_serde::encode::Error) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}