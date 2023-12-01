use {
	crate::{
		headers::ApiKey, middleware::auth::jwt::GameServerInfo, res::responses, Error, Result,
		State,
	},
	axum_extra::TypedHeader,
	jsonwebtoken as jwt,
	std::net::{Ipv4Addr, SocketAddr},
	tokio::net::UdpSocket,
	tracing::debug,
};

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

	let server_info = GameServerInfo::new(server.id);
	let token = jwt::encode(&state.jwt.header, &server_info, &state.jwt.encode)?;
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