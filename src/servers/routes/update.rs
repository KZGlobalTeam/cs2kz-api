use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::responses::NoContent;
use crate::servers::ServerUpdate;
use crate::{audit, responses, AppState, Error, Result};

/// Update a registered CS2 server.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Servers",
  path = "/servers/{server_id}",
  params(("server_id" = u16, Path, description = "The server's ID")),
  request_body = ServerUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["servers"]),
  ),
)]
pub async fn update(
	state: AppState,
	Path(server_id): Path<u16>,
	Json(server_update): Json<ServerUpdate>,
) -> Result<NoContent> {
	let mut query = QueryBuilder::new("UPDATE Servers");
	let mut delimiter = " SET ";

	if let Some(ref name) = server_update.name {
		query.push(delimiter).push(" name = ").push_bind(name);

		delimiter = ",";
	}

	if let Some(ref ip_address) = server_update.ip_address {
		query
			.push(delimiter)
			.push(" ip_address = ")
			.push_bind(ip_address.ip().to_string())
			.push(", port = ")
			.push_bind(ip_address.port());

		delimiter = ",";
	}

	if let Some(steam_id) = server_update.owned_by {
		query
			.push(delimiter)
			.push(" owned_by = ")
			.push_bind(steam_id);
	}

	query.push(" WHERE id = ").push_bind(server_id);

	let result = query.build().execute(&state.database).await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown("server ID").with_detail(server_id));
	}

	audit!("updated server", id = %server_id, update = ?server_update);

	Ok(NoContent)
}
