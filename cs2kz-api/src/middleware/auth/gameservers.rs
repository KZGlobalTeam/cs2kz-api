use {
	super::jwt::GameServerInfo,
	crate::{middleware, Error, Result, State},
	axum::{
		body::Body,
		extract::ConnectInfo,
		headers::{authorization::Bearer, Authorization},
		http::Request,
		middleware::Next,
		response::Response,
		TypedHeader,
	},
	chrono::Utc,
	jsonwebtoken as jwt,
	serde::Deserialize,
	std::net::{IpAddr, Ipv4Addr, SocketAddr},
};

#[derive(Debug, Deserialize)]
struct ServerMetadata {
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
	TypedHeader(api_token): TypedHeader<Authorization<Bearer>>,
	request: Request<Body>,
	next: Next<Body>,
) -> Result<Response> {
	let server_info = jwt::decode::<GameServerInfo>(
		api_token.token(),
		&state.jwt().decode,
		&state.jwt().validation,
	)?
	.claims;

	let server = sqlx::query! {
		r#"
		SELECT
			id,
			ip_address,
			port AS `port: u16`
		FROM
			Servers
		WHERE
			id = ?
			AND api_token IS NOT NULL
		"#,
		server_info.id,
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	if server_info.expires_on < Utc::now() {
		return Err(Error::Unauthorized);
	}

	let ip_address = server
		.ip_address
		.parse::<Ipv4Addr>()
		.map(IpAddr::V4)
		.expect("invalid ip address in database");

	if addr.ip() != ip_address {
		return Err(Error::Unauthorized);
	}

	let (metadata, mut request) = middleware::extract_from_body::<ServerMetadata>(request).await?;

	request
		.extensions_mut()
		.insert(Some(AuthenticatedServer {
			id: server.id,
			plugin_version: metadata.plugin_version,
		}));

	Ok(next.run(request).await)
}
