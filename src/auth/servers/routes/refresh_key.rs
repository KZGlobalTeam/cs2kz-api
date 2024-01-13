use axum::Json;
use chrono::{Duration, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::servers::{AuthResponse, AuthenticatedServer};
use crate::auth::JWT;
use crate::extractors::State;
use crate::responses::{self, Created};
use crate::{Error, Result};

/// Request payload for CS2 servers which want to refresh their access token.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthRequest {
	/// The server's semi-permanent API Key.
	pub refresh_token: u32,

	/// The cs2kz version the server is currently running on.
	#[schema(value_type = String)]
	pub plugin_version: Version,
}

/// This route is used by CS2 servers for refreshing their JWTs.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/servers/refresh",
  request_body = AuthRequest,
  responses(
    responses::Created<AuthResponse>,
    responses::UnprocessableEntity,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
)]
pub async fn refresh_key(
	state: State,
	Json(auth): Json<AuthRequest>,
) -> Result<Created<Json<AuthResponse>>> {
	let data = sqlx::query! {
		r#"
		SELECT
		  s.id server_id,
		  v.id plugin_version_id
		FROM
		  Servers s
		  JOIN PluginVersions v ON v.version = ?
		  AND s.api_key = ?
		"#,
		auth.plugin_version.to_string(),
		auth.refresh_token,
	}
	.fetch_optional(state.database())
	.await?
	.ok_or(Error::Unauthorized)?;

	let payload = AuthenticatedServer::new(data.server_id, data.plugin_version_id);
	let expires_on = Utc::now() + Duration::minutes(30);
	let jwt = JWT::new(payload, expires_on);
	let access_token = state.encode_jwt(&jwt)?;

	Ok(Created(Json(AuthResponse { access_token })))
}
