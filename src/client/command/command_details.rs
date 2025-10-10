use std::fmt::{Debug, Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};
use indexmap::IndexMap;
use log::warn;
use crate::client::command::Command;

#[derive(Clone, Debug)]
pub enum CommandState {
    WAITING, RUNNING, COMPLETED, ABORTED,
}

impl Display for CommandState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct CommandDetails {
    name: String,
    properties: IndexMap<String, String>,
    state: CommandState,
    waiting_at: u128,
    running_at: Option<u128>,
    terminated_at: Option<u128>,
}

impl CommandDetails {
    pub fn new(command: &Box<dyn Command>) -> Self {
        let name = command.get_name();
        let properties = command.get_properties();
        let state = CommandState::WAITING;
        let waiting_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let running_at = None;
        let terminated_at = None;
        Self {name, properties, state, waiting_at, running_at, terminated_at }
    }

    pub fn set_state(&mut self, state: CommandState) {
        self.state = state;
        match self.state {
            CommandState::RUNNING =>
                self.running_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
            CommandState::COMPLETED | CommandState::ABORTED =>
                self.terminated_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
            _ => warn!("can only change command state to running, completed or aborted"),
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn properties(&self) -> &IndexMap<String, String> {
        &self.properties
    }

    pub fn state(&self) -> &CommandState {
        &self.state
    }

    pub fn waiting_at(&self) -> u128 {
        self.waiting_at
    }

    pub fn running_at(&self) -> Option<u128> {
        self.running_at
    }

    pub fn terminated_at(&self) -> Option<u128> {
        self.terminated_at
    }
}

impl Display for CommandDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut properties = String::new();
        self.properties.iter().for_each(|(k, v)| properties += &format!("{}: {},", k, v).as_str());
        let running_at = match self.running_at {
            Some(i) => format!("{}", i),
            None => "".to_string(),
        };
        let terminated_at = match self.terminated_at {
            Some(i) => format!("{}", i),
            None => "".to_string(),
        };

        write!(f,
               "name: [{}], properties: [{}], state: [{}], waiting_at: [{}], running_at: [{}], terminated_at: [{}]",
               self.name, properties, self.state, self.waiting_at, running_at, terminated_at
        )
    }
}
