use axum::extract::Path;

use crate::responses::{self, NoContent};
use crate::{audit, AppState, Error, Result};

/// Delete a CS2 server's API key.
///
/// The server owner cannot generate a new one, so this effectively disables their server until an
/// admin generates a new key.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  tag = "Servers",
  path = "/servers/{server_id}/key",
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["servers"]),
  ),
)]
pub async fn delete_key(state: AppState, Path(server_id): Path<u16>) -> Result<NoContent> {
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
	.execute(&state.database)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown_id("server", server_id));
	}

	audit!("deleted API key for server", id = %server_id);

	Ok(NoContent)
}
