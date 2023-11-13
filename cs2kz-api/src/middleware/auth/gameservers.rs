use {
	crate::{util, Error, Result, State},
	axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response},
	serde::Deserialize,
	std::net::{IpAddr, Ipv4Addr, SocketAddr},
};

#[derive(Debug, Deserialize)]
struct ServerMetadata {
	port: u16,
	plugin_version: u16,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedServer {
	pub id: u16,
	pub plugin_version: u16,
}

#[tracing::instrument(level = "DEBUG")]
pub async fn auth_server(
	state: State,
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	request: Request<Body>,
	next: Next<Body>,
) -> Result<Response> {
	let api_key = request
		.headers()
		.get("api-key")
		.ok_or(Error::MissingApiKey)?
		.to_str()
		.map_err(|_| Error::InvalidApiKey)?
		.parse::<u32>()
		.map_err(|_| Error::InvalidApiKey)?;

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

	let ip_address = server
		.ip_address
		.parse::<Ipv4Addr>()
		.map(IpAddr::V4)
		.expect("invalid ip address in database");

	if addr.ip() != ip_address {
		return Err(Error::Unauthorized);
	}

	let (metadata, mut request) = util::extract_from_body::<ServerMetadata>(request).await?;

	if metadata.port != server.port {
		return Err(Error::Unauthorized);
	}

	// TODO(AlphaKeks): send the server a UDP packet or something

	request
		.extensions_mut()
		.insert(Some(AuthenticatedServer {
			id: server.id,
			plugin_version: metadata.plugin_version,
		}));

	Ok(next.run(request).await)
}
