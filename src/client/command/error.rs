use std::fmt::Debug;
use autonomi::client::{ConnectError, PutError};
use autonomi::graph::GraphError;
use autonomi::pointer::PointerError;
use autonomi::register::RegisterError;
use autonomi::scratchpad::ScratchpadError;
use thiserror::Error;
use crate::error::chunk_error::ChunkError;

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

impl From<ChunkError> for CommandError {
    fn from(value: ChunkError) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<GraphError> for CommandError {
    fn from(value: GraphError) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<PointerError> for CommandError {
    fn from(value: PointerError) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<PutError> for CommandError {
    fn from(value: PutError) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<RegisterError> for CommandError {
    fn from(value: RegisterError) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<ScratchpadError> for CommandError {
    fn from(value: ScratchpadError) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}

impl From<rmp_serde::encode::Error> for CommandError {
    fn from(value: rmp_serde::encode::Error) -> Self {
        Self::Unrecoverable(value.to_string())
    }
}