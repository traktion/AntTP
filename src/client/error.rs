use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use crate::client::command::Command;

#[derive(Error, Debug)]
pub enum ChunkError {
    #[error("create error: {0}")]
    CreateError(SendError<Box<dyn Command>>),
    #[error("get error: {0}")]
    GetError(GetError),
}

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("create error: {0}")]
    CreateError(SendError<Box<dyn Command>>),
    #[error("update error: {0}")]
    UpdateError(SendError<Box<dyn Command>>),
    #[error("get error: {0}")]
    GetError(GetError),
}

#[derive(Error, Debug)]
pub enum PointerError {
    #[error("create error: {0}")]
    CreateError(SendError<Box<dyn Command>>),
    #[error("update error: {0}")]
    UpdateError(SendError<Box<dyn Command>>),
    #[error("get error: {0}")]
    GetError(GetError),
    #[error("check error: {0}")]
    CheckError(CheckError),
}

#[derive(Error, Debug)]
pub enum PublicDataError {
    #[error("create error: {0}")]
    CreateError(SendError<Box<dyn Command>>),
    #[error("get error: {0}")]
    GetError(GetError),
}

#[derive(Error, Debug)]
pub enum RegisterError {
    #[error("create error: {0}")]
    CreateError(SendError<Box<dyn Command>>),
    #[error("update error: {0}")]
    UpdateError(SendError<Box<dyn Command>>),
    #[error("get error: {0}")]
    GetError(GetError),
}

#[derive(Error, Debug)]
pub enum ScratchpadError {
    #[error("create error: {0}")]
    CreateError(SendError<Box<dyn Command>>),
    #[error("update error: {0}")]
    UpdateError(SendError<Box<dyn Command>>),
    #[error("get error: {0}")]
    GetError(GetError),
}

#[derive(Error, Debug)]
pub enum GetError {
    #[error("network error: {0}")]
    RecordNotFound(String),
    #[error("bad address error: {0}")]
    BadAddress(String),
    #[error("address not derived from: {0}")]
    NotDerivedAddress(String),
    #[error("derivation name missing: {0}")]
    DerivationNameMissing(String),
    #[error("derivation key missing: {0}")]
    DerivationKeyMissing(String),
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("network is offline: {0}")]
    NetworkOffline(String),
    #[error("streaming error: {0}")]
    StreamingError(String),
}

#[derive(Error, Debug)]
pub enum CheckError {
    #[error("network error: {0}")]
    RecordNotFound(String),
}
