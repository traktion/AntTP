use async_trait::async_trait;
use indexmap::IndexMap;
use crate::client::command::error::CommandError;

#[async_trait]
pub trait Command: Send {
    async fn execute(&self) -> Result<(), CommandError>;

    fn action_hash(&self) -> Vec<u8>;

    fn id(&self) -> u128;

    fn name(&self) -> String {
        "Command".to_string()
    }

    fn properties(&self) -> IndexMap<String, String> {
        IndexMap::new()
    }
}

pub mod pointer;
pub mod register;
pub mod executor;
pub mod chunk;
pub mod public_data;
pub mod error;
pub mod graph;
pub mod scratchpad;
pub mod command_details;
pub mod access_list;