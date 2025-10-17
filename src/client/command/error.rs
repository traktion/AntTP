use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("unrecoverable error: {0}")]
    Unrecoverable(String),
    #[error("recoverable error: {0}")]
    Recoverable(String),
}
