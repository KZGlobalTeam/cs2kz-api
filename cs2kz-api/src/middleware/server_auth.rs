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
	crate::{Error, Result, State},
	axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response},
	serde_json::Value as JsonValue,
	std::net::Ipv4Addr,
};

pub const API_KEY_HEADER: &str = "api-key";
pub const API_TOKEN_HEADER: &str = "api-token";

#[derive(Debug, Clone)]
pub struct ServerData {
	pub id: u16,
	pub plugin_version: u16,
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
pub async fn verify_api_key(
	state: State,
	ConnectInfo(ip): ConnectInfo<Ipv4Addr>,
	request: Request<Body>,
	next: Next<Body>,
) -> Result<Response> {
	let api_key = request
		.headers()
		.get(API_KEY_HEADER)
		.ok_or(Error::MissingApiKey)?
		.to_str()
		.map_err(|_| Error::InvalidApiKey)?
		.parse::<u32>()
		.map_err(|_| Error::InvalidApiKey)?;

	sqlx::query! {
		r#"
		SELECT
			id
		FROM
			Servers
		WHERE
			api_key = ?
			AND ip_address = ?
		"#,
		api_key,
		ip.to_string(),
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	Ok(next.run(request).await)
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
pub async fn verify_api_token(
	state: State,
	ConnectInfo(ip): ConnectInfo<Ipv4Addr>,
	request: Request<Body>,
	next: Next<Body>,
) -> Result<Response> {
	let headers = request.headers();

	// Extract API key.
	// This is required as every server has one.
	let api_key = headers
		.get(API_KEY_HEADER)
		.ok_or(Error::MissingApiKey)?
		.to_str()
		.map_err(|_| Error::InvalidApiKey)?
		.parse::<u32>()
		.map_err(|_| Error::InvalidApiKey)?;

	// Extract API token.
	// This might not exist if the server hasn't generated one yet.
	// If it provided one though, it needs to be formatted properly.
	let api_token = match headers.get(API_TOKEN_HEADER) {
		None => None,
		Some(token) => token
			.to_str()
			.map_err(|_| Error::InvalidApiToken)?
			.parse::<u32>()
			.map(Some)
			.map_err(|_| Error::InvalidApiToken)?,
	};

	// Extract plugin version information from the request body.
	let (parts, body) = request.into_parts();
	let (mut json, body) = helpers::deserialize_body::<JsonValue>(body).await?;
	let plugin_version = serde_json::from_value::<u16>(json["plugin_version"].take())
		.map_err(|_| Error::InvalidRequestBody)?;

	// Select a set of valid plugin versions.
	// Right now this is just the latest version, but maybe we can relax it a bit?
	let mut valid_versions = sqlx::query!("SELECT MAX(id) version FROM PluginVersions")
		.fetch_all(state.database())
		.await?
		.into_iter()
		.filter_map(|row| row.version);

	if !valid_versions.any(|version| version == plugin_version) {
		return Err(Error::OutdatedPluginVersion);
	}

	// Fetch the server's ID and `current_token` so we can forward the ID to the next service.
	// We will also want to check `current_token` to potentially update its expiration date and
	// replace it with the `api_token` we just got, if they are different.
	//
	// Since `current_token` and `token_expires_at` are tied together, and we check the
	// expiration date, we can safely accept either token matching.
	//
	// We want to let this request through if
	//
	// 1) `current_token` is NULL => this is the very first request; we do *not* want to check
	//    the expiration date (there will be none yet).
	// 2) either of the two tokens match *and* the expiration date is valid
	let server = sqlx::query! {
		r#"
		SELECT
			id,
			current_token,
			next_token
		FROM
			Servers
		WHERE
			api_key = ?
			AND ip_address = ?
			AND (
				current_token IS NULL
				OR (
					(
						current_token = ?
						OR next_token = ?
					)
					AND CURRENT_TIMESTAMP() < token_expires_at
				)
			)
		"#,
		api_key,
		ip.to_string(),
		api_token,
		api_token,
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	let should_update_token = match (api_token, server.current_token, server.next_token) {
		// This server has no token yet.
		// This means this request must be its very first request, so we need to update
		// `current_token` and the expiration date.
		(None, None, _) => true,

		// The server does not have a token yet, but provided one as part of the request...
		//
		// probably fine?
		(Some(_), None, _) => true,

		// The server has a token, but the request didn't!
		(None, Some(_), _) => return Err(Error::Unauthorized),

		// This server has a token and the request headers also had one, so we only want to
		// update the token information if they're different.
		//
		// TODO: not entirely sure this is correct
		(Some(token), Some(current_token), Some(next_token)) => {
			token == next_token && next_token != current_token
		}

		_ => return Err(Error::Unauthorized),
	};

	if should_update_token {
		sqlx::query! {
			r#"
			UPDATE
				Servers
			SET
				current_token = next_token,
				token_expires_at = DATE_ADD(CURRENT_TIMESTAMP(), INTERVAL 1 HOUR)
			WHERE
				id = ?
			"#,
			server.id,
		}
		.execute(state.database())
		.await?;
	}

	sqlx::query! {
		r#"
		UPDATE
			Servers
		SET
			token_last_used_at = CURRENT_TIMESTAMP()
		WHERE
			id = ?
		"#,
		server.id,
	}
	.execute(state.database())
	.await?;

	let mut request = Request::from_parts(parts, body);

	request
		.extensions_mut()
		.insert(ServerData { id: server.id, plugin_version });

	Ok(next.run(request).await)
}
