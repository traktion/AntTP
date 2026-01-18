use actix_web::Error;
use actix_web::web::Data;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use utoipa::ToSchema;
use crate::client::command::command_details::CommandDetails;

#[derive(ToSchema, Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub properties: Vec<Property>,
    pub state: String,
    pub waiting_at: u128,
    pub running_at: Option<u128>,
    pub terminated_at: Option<u128>,
}

impl Command {
    pub fn new(id: String, name: String, properties: Vec<Property>, state: String, waiting_at: u128,
               running_at: Option<u128>, terminated_at: Option<u128>) -> Self {
        Self { id, name, properties, state, waiting_at, running_at, terminated_at }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: String,
}

impl Property {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

#[derive(utoipa::ToResponse, Serialize, Deserialize, Debug, Clone)]
pub struct CommandList(pub Vec<Command>);

#[derive(Debug)]
pub struct CommandService {
    commands_map: Data<Mutex<IndexMap<u128, CommandDetails>>>,
}

impl CommandService {
    pub fn new(commands_map: Data<Mutex<IndexMap<u128, CommandDetails>>>) -> Self {
        Self { commands_map }
    }

    pub async fn get_commands(&self) -> Result<CommandList, Error> {
        let commands_map = self.commands_map.get_ref().lock().await;
        let mut commands = Vec::<Command>::with_capacity(commands_map.len());

        commands_map.values().for_each(|v|{
            let mut properties = Vec::<Property>::with_capacity(v.properties().len());
            v.properties().iter().for_each(|(k, v)|properties.push(Property::new(k.clone(), v.clone())));
            commands.push(
                Command::new(v.id().to_string(), v.name().clone(), properties, v.state().to_string(),
                             v.waiting_at(), v.running_at(), v.terminated_at())
            )
        });

        Ok(CommandList(commands))
    }
}