use {
	crate::{Error, Result, State},
	axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response},
	std::net::{IpAddr, Ipv4Addr, SocketAddr},
};

pub struct AuthenticatedServer {
	pub id: u16,
}

#[tracing::instrument(level = "DEBUG")]
pub async fn auth_server(
	state: State,
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	mut request: Request<Body>,
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

	let server = sqlx::query!("SELECT * FROM Servers WHERE api_key = ?", api_key)
		.fetch_one(state.database())
		.await
		.map_err(|_| Error::Unauthorized)?;

	let ip_address = server
		.ip_address
		.parse::<Ipv4Addr>()
		.map(IpAddr::V4)
		.expect("invalid ip address in database");

	if ip_address != addr.ip() {
		return Err(Error::Unauthorized);
	}

	request
		.extensions_mut()
		.insert(AuthenticatedServer { id: server.id });

	Ok(next.run(request).await)
}
