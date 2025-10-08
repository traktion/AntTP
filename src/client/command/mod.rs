use crate::client::command::error::CommandError;
use async_trait::async_trait;

#[async_trait]
pub trait Command: Send {
    async fn execute(&self) -> Result<(), CommandError>;
    fn get_hash(&self) -> Vec<u8>;
}

pub mod pointer;
pub mod register;
pub mod executor;
pub mod chunk;
pub mod public_data;
pub mod error;
pub mod graph;
pub mod scratchpad;