use axum::Json;
use tracing::trace;

use crate::auth::servers::{AccessToken, RefreshToken};
use crate::auth::Server;
use crate::responses::Created;
use crate::{responses, AppState, Error, Result};

/// Create a new JWT for authenticating CS2 server requests.
///
/// This endpoint will be used by CS2 servers while they are running.
/// Each token is only valid for 30 minutes.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Servers",
  path = "/servers/key",
  request_body = RefreshToken,
  responses(
    responses::Created<AccessToken>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn create_jwt(
	state: AppState,
	Json(refresh): Json<RefreshToken>,
) -> Result<Created<Json<AccessToken>>> {
	let server = sqlx::query! {
		r#"
		SELECT
		  s.id server_id,
		  v.id plugin_version_id
		FROM
		  Servers s
		  JOIN PluginVersions v ON v.version = ?
		  AND s.api_key = ?
		"#,
		refresh.plugin_version.to_string(),
		refresh.key,
	}
	.fetch_optional(&state.database)
	.await?
	.map(|row| Server::new(row.server_id, row.plugin_version_id))
	.ok_or_else(|| {
		trace!(?refresh, "invalid refresh token");
		Error::invalid("API key").unauthorized()
	})?;

	server
		.into_jwt(&state.jwt)
		.map(Json)
		.map(Created)
		.map_err(Error::from)
}
