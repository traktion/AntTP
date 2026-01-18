use tonic::{Request, Response, Status};
use actix_web::web::Data;
use crate::service::command_service::{CommandService, Command as ServiceCommand, Property as ServiceProperty, CommandList as ServiceCommandList};

pub mod command_proto {
    tonic::include_proto!("command");
}

use command_proto::command_service_server::CommandService as CommandServiceTrait;
pub use command_proto::command_service_server::CommandServiceServer;
use command_proto::{Command, Property, CommandList, GetCommandsRequest};

pub struct CommandHandler {
    command_service: Data<CommandService>,
}

impl CommandHandler {
    pub fn new(command_service: Data<CommandService>) -> Self {
        Self { command_service }
    }
}

impl From<ServiceProperty> for Property {
    fn from(p: ServiceProperty) -> Self {
        Property {
            name: p.name,
            value: p.value,
        }
    }
}

impl From<ServiceCommand> for Command {
    fn from(c: ServiceCommand) -> Self {
        Command {
            id: c.id,
            name: c.name,
            properties: c.properties.into_iter().map(Property::from).collect(),
            state: c.state,
            waiting_at: c.waiting_at as u64,
            running_at: c.running_at.map(|v| v as u64),
            terminated_at: c.terminated_at.map(|v| v as u64),
        }
    }
}

impl From<ServiceCommandList> for CommandList {
    fn from(cl: ServiceCommandList) -> Self {
        CommandList {
            commands: cl.0.into_iter().map(Command::from).collect(),
        }
    }
}

#[tonic::async_trait]
impl CommandServiceTrait for CommandHandler {
    async fn get_commands(
        &self,
        _request: Request<GetCommandsRequest>,
    ) -> Result<Response<CommandList>, Status> {
        let result = self.command_service.get_commands().await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CommandList::from(result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;
    use indexmap::IndexMap;
    use crate::client::command::command_details::CommandDetails;
    use crate::service::command_service::Property as ServiceProperty;

    #[tokio::test]
    async fn test_get_commands() {
        let commands_map = Data::new(Mutex::new(IndexMap::<u128, CommandDetails>::new()));
        let command_service = Data::new(CommandService::new(commands_map.clone()));
        let handler = CommandHandler::new(command_service);

        let request = Request::new(GetCommandsRequest {});
        let response = handler.get_commands(request).await.unwrap();

        assert_eq!(response.into_inner().commands.len(), 0);
    }

    #[test]
    fn test_property_from_service() {
        let service_property = ServiceProperty::new("test_name".to_string(), "test_value".to_string());
        let proto_property = Property::from(service_property);
        assert_eq!(proto_property.name, "test_name");
        assert_eq!(proto_property.value, "test_value");
    }

    #[test]
    fn test_command_from_service() {
        let service_command = ServiceCommand::new(
            "id1".to_string(),
            "name1".to_string(),
            vec![ServiceProperty::new("p1".to_string(), "v1".to_string())],
            "state1".to_string(),
            100,
            Some(200),
            None,
        );
        let proto_command = Command::from(service_command);
        assert_eq!(proto_command.id, "id1");
        assert_eq!(proto_command.name, "name1");
        assert_eq!(proto_command.properties.len(), 1);
        assert_eq!(proto_command.properties[0].name, "p1");
        assert_eq!(proto_command.state, "state1");
        assert_eq!(proto_command.waiting_at, 100);
        assert_eq!(proto_command.running_at, Some(200));
        assert_eq!(proto_command.terminated_at, None);
    }
}
