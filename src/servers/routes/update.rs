use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::extractors::State;
use crate::servers::ServerUpdate;
use crate::{responses, Result};

/// Update a server.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Servers",
  path = "/servers",
  params(("server_id" = u16, Path, description = "The server's ID")),
  request_body = ServerUpdate,
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["servers_edit"]),
  ),
)]
pub async fn update(
	state: State,
	Path(server_id): Path<u16>,
	Json(server_update): Json<ServerUpdate>,
) -> Result<()> {
	let mut query = QueryBuilder::new("UPDATE Servers");
	let mut delimiter = " SET ";

	if let Some(ref name) = server_update.name {
		query.push(delimiter).push_bind(" name = ").push_bind(name);

		delimiter = ",";
	}

	if let Some(ref ip_address) = server_update.ip_address {
		query
			.push(delimiter)
			.push_bind(" ip_address = ")
			.push_bind(ip_address.ip().to_string())
			.push(", port = ")
			.push_bind(ip_address.port());

		delimiter = ",";
	}

	if let Some(steam_id) = server_update.owned_by {
		query
			.push(delimiter)
			.push_bind(" owned_by = ")
			.push_bind(steam_id);
	}

	query.build().execute(state.database()).await?;

	Ok(())
}
