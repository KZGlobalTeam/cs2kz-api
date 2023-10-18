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
pub async fn verify_server(
	state: State,
	ConnectInfo(ip): ConnectInfo<Ipv4Addr>,
	request: Request<Body>,
	next: Next<Body>,
) -> Result<Response> {
	let headers = request.headers();

	let api_key = headers
		.get(API_KEY_HEADER)
		.ok_or(Error::MissingApiKey)?
		.to_str()
		.map_err(|_| Error::InvalidApiKey)?
		.parse::<u32>()
		.map_err(|_| Error::InvalidApiKey)?;

	let api_token = headers
		.get(API_TOKEN_HEADER)
		.ok_or(Error::MissingApiToken)?
		.to_str()
		.map_err(|_| Error::InvalidApiToken)?
		.parse::<u32>()
		.map_err(|_| Error::InvalidApiToken)?;

	let (parts, body) = request.into_parts();
	let (mut json, body) = helpers::deserialize_body::<JsonValue>(body).await?;
	let plugin_version = serde_json::from_value::<u16>(json["plugin_version"].take())
		.map_err(|_| Error::InvalidRequestBody)?;

	// TODO: maybe allow latest 3 versions? or any version released in the last week?
	let mut valid_versions = sqlx::query!("SELECT MAX(id) version FROM PluginVersions")
		.fetch_all(state.database())
		.await?
		.into_iter()
		.filter_map(|row| row.version);

	if !valid_versions.any(|version| version == plugin_version) {
		return Err(Error::OutdatedPluginVersion);
	}

	let id = sqlx::query! {
		r#"
		SELECT
			id
		FROM
			Servers
		WHERE
			api_key = ?
			AND ip_address = ?
			AND (
				current_token = ?
				OR next_token = ?
			)
			AND CURRENT_TIMESTAMP() < token_expires_at
		"#,
		api_key,
		ip.to_string(),
		api_token,
		api_token,
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?
	.id;

	sqlx::query! {
		r#"
		UPDATE
			Servers
		SET
			current_token = ?,
			token_last_used_at = CURRENT_TIMESTAMP()
		WHERE
			id = ?
		"#,
		api_token,
		id,
	}
	.execute(state.database())
	.await?;

	let mut request = Request::from_parts(parts, body);

	request
		.extensions_mut()
		.insert(ServerData { id, plugin_version });

	Ok(next.run(request).await)
}
