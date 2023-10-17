// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	super::helpers,
	crate::{state::AppState, Error, Result, State},
	axum::{
		body::Body,
		extract::ConnectInfo,
		http::Request,
		middleware::Next,
		response::{IntoResponse, Response},
	},
	serde_json::Value as JsonValue,
	sqlx::types::Uuid,
	std::net::Ipv4Addr,
};

#[derive(Debug, Clone)]
pub struct ServerData {
	pub id: u16,
	pub ip: Ipv4Addr,
	pub port: u16,
	pub token: Uuid,
}

#[tracing::instrument(
	level = "DEBUG",
	skip_all,
	fields(
		route = %request.uri().path(),
		method = %request.method(),
		ip = %ip,
	),
)]
pub async fn verify_server(
	state: State,
	ConnectInfo(ip): ConnectInfo<Ipv4Addr>,
	request: Request<Body>,
	next: Next<Body>,
) -> Response {
	match verify_server_inner(*state, ip, request).await {
		Ok(request) => next.run(request).await,
		Err(error) => error.into_response(),
	}
}

async fn verify_server_inner(
	state: &AppState,
	ip: Ipv4Addr,
	request: Request<Body>,
) -> Result<Request<Body>> {
	let header = "api-token";

	// Extract API token from the request headers
	let token = request
		.headers()
		.get(header)
		.ok_or(Error::MissingToken)?
		.to_str()
		.map_err(|_| Error::InvalidToken)?
		.parse::<Uuid>()
		.map_err(|_| Error::InvalidToken)?;

	// extract request body
	let (parts, body) = request.into_parts();

	// deserialize body as json
	let (mut json, body) = helpers::deserialize_body::<JsonValue>(body).await?;

	// reconstruct request body
	let mut request = Request::from_parts(parts, body);

	// extract server port from the json
	let port = serde_json::from_value::<u16>(json["port"].take())
		.map_err(|_| Error::InvalidRequestBody)?;

	// find server that matches the token, ip and port and get its ID
	let server = sqlx::query! {
		r#"
		SELECT
			id
		FROM
			Servers s
			JOIN ServerOwners so ON so.server_id = s.id
		WHERE
			so.token = ?
			AND s.ip_address = ?
			AND s.port = ?
		"#,
		token,
		ip.to_string(),
		port,
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	// make sure handlers have access to the verified data
	request
		.extensions_mut()
		.insert(ServerData { id: server.id, ip, port, token });

	Ok(request)
}
