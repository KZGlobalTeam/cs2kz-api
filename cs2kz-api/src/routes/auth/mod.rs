//! This module holds all HTTP handlers related to authentication.

use axum::routing::post;
use axum::{Json, Router};
use semver::Version;
use serde::{Deserialize, Serialize};
use tracing::trace;
use utoipa::ToSchema;

use crate::jwt::ServerClaims;
use crate::responses::Created;
use crate::{openapi as R, AppState, Error, Result, State};

pub mod steam;

/// This function returns the router for the `/auth` routes.
pub fn router(state: &'static AppState) -> Router {
	Router::new()
		.route("/refresh", post(refresh_token))
		.with_state(state)
		.nest("/steam", steam::router(state))
}

/// This endpoint is used by servers to refresh their JWTs.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Auth",
	path = "/auth/refresh",
	request_body = AuthRequest,
	responses(
		R::Created<AuthResponse>,
		R::BadRequest,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn refresh_token(
	state: State,
	Json(body): Json<AuthRequest>,
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
		body.plugin_version.to_string(),
		body.api_key,
	}
	.fetch_optional(state.database())
	.await?
	.ok_or(Error::Unauthorized)?;

	let claims = ServerClaims::new(data.server_id, data.plugin_version_id);
	let token = state.encode_jwt(&claims)?;

	trace!(%token, %data.server_id, "generated JWT for server");

	Ok(Created(Json(AuthResponse { token })))
}

/// This data is sent by servers to refresh their JWT.
#[derive(Debug, Deserialize, ToSchema)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct AuthRequest {
	/// The server's "permanent" API key.
	pub(crate) api_key: u32,

	/// The CS2KZ version the server is currently running.
	#[schema(value_type = String)]
	pub(crate) plugin_version: Version,
}

/// This data is sent by servers to refresh their JWT.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct AuthResponse {
	pub(crate) token: String,
}
