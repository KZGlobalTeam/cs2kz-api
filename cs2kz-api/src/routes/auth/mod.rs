//! This module holds all HTTP handlers related to authentication.

use std::io::{self, ErrorKind as IoError};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::routing::get;
use axum::{Json, Router};
use semver::Version;
use serde::Deserialize;
use tokio::net::UdpSocket;
use tracing::trace;
use utoipa::ToSchema;

use crate::jwt::ServerClaims;
use crate::responses::Created;
use crate::{openapi as R, AppState, Error, Result, State};

pub mod steam;

/// This function returns the router for the `/auth` routes.
pub fn router(state: &'static AppState) -> Router {
	Router::new()
		.route("/refresh", get(refresh_token))
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
		R::Ok<()>,
		R::BadRequest,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn refresh_token(state: State, Json(body): Json<AuthRequest>) -> Result<Created<()>> {
	let server = sqlx::query! {
		r#"
		SELECT
			id,
			ip_address,
			port
		FROM
			Servers
		WHERE
			api_key = ?
		"#,
		body.api_key,
	}
	.fetch_optional(state.database())
	.await?
	.ok_or(Error::Unauthorized)?;

	let claims = ServerClaims::new(server.id, body.plugin_version);
	let token = state.encode_jwt(&claims)?;
	let socket = UdpSocket::bind("127.0.0.1:0")
		.await
		.map_err(|err| Error::Unexpected(Box::new(err)))?;

	let server_addr = server
		.ip_address
		.parse::<Ipv4Addr>()
		.map(|ip| SocketAddr::new(IpAddr::V4(ip), server.port))
		.map_err(|err| Error::Unexpected(Box::new(err)))?;

	let map_err = |err: io::Error| match err.kind() {
		// If we get any of these it means that the server we expected is either down or
		// disfunctional, so we'll just count that as "unauthorized".
		IoError::NotFound
		| IoError::ConnectionRefused
		| IoError::ConnectionReset
		| IoError::ConnectionAborted
		| IoError::TimedOut => Error::Unauthorized,

		// Anything else is our fault.
		_ => Error::Unexpected(Box::new(err)),
	};

	socket.connect(server_addr).await.map_err(map_err)?;
	socket.send(token.as_bytes()).await.map_err(map_err)?;

	trace!(server_id = %server.id, %server_addr, %token, "sent JWT to server");

	Ok(Created(()))
}

/// This data is sent by servers to refresh their JWT.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthRequest {
	/// The server's "permanent" API key.
	api_key: u32,

	/// The CS2KZ version the server is currently running.
	#[schema(value_type = String)]
	plugin_version: Version,
}
