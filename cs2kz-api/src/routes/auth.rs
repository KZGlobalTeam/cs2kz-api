use {
	crate::{headers::ApiKey, middleware::auth::jwt::GameServerInfo, Error, Result, State},
	axum::TypedHeader,
	jsonwebtoken as jwt,
	std::net::{IpAddr, Ipv4Addr, SocketAddr},
	tokio::net::UdpSocket,
	tracing::error,
};

pub async fn refresh_token(
	state: State,
	TypedHeader(ApiKey(api_key)): TypedHeader<ApiKey>,
) -> Result<()> {
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
		api_key
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	let server_info = GameServerInfo::new(server.id);
	let token = jwt::encode(&state.jwt().header, &server_info, &state.jwt().encode)?;

	let socket = UdpSocket::bind("0.0.0.0:0")
		.await
		.map_err(|_| Error::InternalServerError)?;

	let ip_address = server
		.ip_address
		.parse::<Ipv4Addr>()
		.expect("invalid ip_address in database");

	let addr = SocketAddr::new(IpAddr::V4(ip_address), server.port);

	socket
		.connect(addr)
		.await
		.map_err(|_| Error::InternalServerError)?;

	if let Err(err) = socket.send(token.as_bytes()).await {
		error!(?err, "Failed to send api_token via UDP");

		return Err(Error::InternalServerError);
	}

	Ok(())
}
