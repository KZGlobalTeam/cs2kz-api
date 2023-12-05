use std::net::{Ipv4Addr, SocketAddr};

use axum_extra::TypedHeader;
use jsonwebtoken as jwt;
use tokio::net::UdpSocket;
use tracing::debug;

use crate::headers::ApiKey;
use crate::middleware::auth::jwt::GameServerInfo;
use crate::res::responses;
use crate::state::JwtState;
use crate::{Error, Result, State};

/// CS2 server authentication.
///
/// This endpoint is used by CS2 game servers to refresh their access token.
#[tracing::instrument(skip(state))]
#[utoipa::path(get, tag = "Auth", context_path = "/api", path = "/auth/token", params(
	("api-key" = u32, Header, description = "API Key"),
), responses(
	responses::Ok<()>,
	responses::BadRequest,
	responses::Unauthorized,
	responses::InternalServerError,
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
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	let JwtState { header, encode, .. } = state.jwt();
	let server_info = GameServerInfo::new(server.id);
	let token = jwt::encode(header, &server_info, encode)?;

	let socket = UdpSocket::bind("127.0.0.0:0")
		.await
		.map_err(|_| Error::InternalServerError)?;

	let server_ip = server
		.ip_address
		.parse::<Ipv4Addr>()
		.expect("invalid IP address in database");

	let server_addr = SocketAddr::from((server_ip, server.port));

	socket
		.connect(server_addr)
		.await
		.map_err(|_| Error::InternalServerError)?;

	// TODO(AlphaKeks): send a header of some sort as well in addition to the token
	socket
		.send(token.as_bytes())
		.await
		.map_err(|_| Error::InternalServerError)?;

	debug!(addr = %server_addr, %token, "sent JWT to server");

	Ok(())
}
