use actix_web::{Error, HttpResponse};
use actix_web::web::Data;
use log::debug;
use crate::service::command_service::{CommandList, CommandService};

#[utoipa::path(
    get,
    path = "/anttp-0/command",
    responses(
        (status = OK, response = CommandList),
    )
)]
pub async fn get_commands(command_service: Data<CommandService>) -> Result<HttpResponse, Error> {
    debug!("Getting command list");
    Ok(HttpResponse::Ok().json(command_service.get_commands().await?))
}
