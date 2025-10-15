use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub enum CommandError {
    Unrecoverable(String),
    Recoverable(String),
}

impl Debug for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            CommandError::Unrecoverable(ref message) => write!(f, "{}", message),
            CommandError::Recoverable(ref message) => write!(f, "{}", message),
        }
    }
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            CommandError::Unrecoverable(ref message) => write!(f, "{}", message),
            CommandError::Recoverable(ref message) => write!(f, "{}", message),
        }
    }
}

impl Error for CommandError {}
