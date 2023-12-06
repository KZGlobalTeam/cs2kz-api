use std::io::{self, ErrorKind as IoError};
use std::net::{Ipv4Addr, SocketAddr};

use axum_extra::TypedHeader;
use tokio::net::UdpSocket;
use tracing::{debug, error};

use crate::headers::ApiKey;
use crate::middleware::auth::jwt::GameServerInfo;
use crate::res::responses;
use crate::{Error, Result, State};

/// CS2 server authentication.
///
/// This endpoint is used by CS2 game servers to refresh their access token.
#[tracing::instrument(skip(state))]
#[utoipa::path(get, tag = "Auth", context_path = "/api", path = "/auth/token", responses(
	responses::Ok<()>,
	responses::BadRequest,
	responses::Unauthorized,
	responses::InternalServerError,
), security(
	("API Key" = []),
))]
pub async fn token(state: State, TypedHeader(ApiKey(api_key)): TypedHeader<ApiKey>) -> Result<()> {
	let server = sqlx::query! {
		r#"
		SELECT
			id,
			ip_address,
			port AS `port: u16`
		FROM
			Servers
		WHERE
			api_key = ?
		"#,
		api_key,
	}
	.fetch_optional(state.database())
	.await?
	.ok_or(Error::Unauthorized)?;

	let server_info = GameServerInfo::new(server.id);
	let token = state.jwt().encode(&server_info)?;

	let socket = UdpSocket::bind("127.0.0.0:0").await.map_err(|err| {
		error!(?err, "failed to bind udp socket");
		Error::InternalServerError
	})?;

	let server_addr = server
		.ip_address
		.parse::<Ipv4Addr>()
		.map(|ip_addr| SocketAddr::from((ip_addr, server.port)))
		.expect("invalid IP address in database");

	let map_err = |err: io::Error| match err.kind() {
		// If we get any of these it means that the server we expected is either down or
		// disfunctional, so we'll just count that as "unauthorized".
		IoError::NotFound
		| IoError::ConnectionRefused
		| IoError::ConnectionReset
		| IoError::ConnectionAborted
		| IoError::TimedOut => Error::Unauthorized,

		// Anything else is our fault.
		_ => Error::InternalServerError,
	};

	socket.connect(server_addr).await.map_err(map_err)?;

	// TODO(AlphaKeks): send a header of some sort as well in addition to the token
	socket.send(token.as_bytes()).await.map_err(map_err)?;

	debug!(addr = %server_addr, %token, "sent JWT to server");

	Ok(())
}
