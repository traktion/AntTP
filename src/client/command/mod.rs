use async_trait::async_trait;
use indexmap::IndexMap;
use crate::client::command::error::CommandError;

#[async_trait]
pub trait Command: Send {
    async fn execute(&self) -> Result<(), CommandError>;

    fn get_action_hash(&self) -> Vec<u8>;

    fn get_id(&self) -> u128;

    fn get_name(&self) -> String {
        "Command".to_string()
    }

    fn get_properties(&self) -> IndexMap<String, String> {
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