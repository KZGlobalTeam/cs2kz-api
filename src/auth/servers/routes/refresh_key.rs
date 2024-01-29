use axum::Json;
use servers::Server;
use tracing::trace;

use crate::auth::servers::{self, AccessToken, RefreshToken};
use crate::responses::{self, Created};
use crate::{AppState, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Auth",
  path = "/auth/servers/refresh_key",
  request_body = RefreshToken,
  responses(
    responses::Created<AccessToken>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn refresh_key(
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
	.fetch_optional(state.database())
	.await?
	.map(|row| Server::new(row.server_id, row.plugin_version_id))
	.ok_or_else(|| {
		trace!(?refresh, "invalid refresh token");
		Error::Unauthorized
	})?;

	server.into_jwt(&state).map(Json).map(Created)
}
