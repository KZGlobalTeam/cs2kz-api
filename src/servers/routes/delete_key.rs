use axum::extract::Path;

use crate::extractors::State;
use crate::{responses, Error, Result};

/// Delete a server's API Key. This effectively deglobals the server until an admin generates a new
/// key again.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  tag = "Servers",
  path = "/servers/{server_id}/key",
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["servers_deglobal"]),
  ),
)]
pub async fn delete_key(state: State, Path(server_id): Path<u16>) -> Result<()> {
	let result = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  api_key = NULL
		WHERE
		  id = ?
		"#,
		server_id,
	}
	.execute(state.database())
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::InvalidServerID(server_id));
	}

	Ok(())
}
