use thiserror::Error;
use serde::Serialize;
use crate::error::GetError;

#[derive(Error, Debug, Serialize)]
pub enum ArchiveError {
    #[error("get error: {0}")]
    GetError(GetError),
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