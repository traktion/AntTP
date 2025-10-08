use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct CommandError {
    message: String
}

impl Debug for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CommandError {}

impl CommandError {
    pub fn from(message: String) -> Self {
        CommandError { message }
    }
}