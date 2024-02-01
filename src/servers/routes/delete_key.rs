use axum::extract::Path;

use crate::{audit, responses, AppState, Error, Result};

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
    ("Steam Session" = ["servers"]),
  ),
)]
pub async fn delete_key(state: AppState, Path(server_id): Path<u16>) -> Result<()> {
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
		return Err(Error::unknown("server ID").with_detail(server_id));
	}

	audit!("deleted API key for server", id = %server_id);

	Ok(())
}
