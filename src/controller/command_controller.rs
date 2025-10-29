use actix_web::{Error, HttpResponse};
use actix_web::web::Data;
use indexmap::IndexMap;
use log::debug;
use tokio::sync::Mutex;
use crate::client::command::command_details::CommandDetails;
use crate::service::command_service::{CommandList, CommandService};

#[utoipa::path(
    get,
    path = "/anttp-0/command",
    responses(
        (status = OK, response = CommandList),
    )
)]
pub async fn get_commands(
    commands_map: Data<Mutex<IndexMap<u128, CommandDetails>>>,
) -> Result<HttpResponse, Error> {
    let command_service = CommandService::new(commands_map.clone());

    debug!("Getting command list");
    Ok(HttpResponse::Ok().json(command_service.get_commands().await?))
}
