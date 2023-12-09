use std::io::{self, ErrorKind as IoError};
use std::net::{Ipv4Addr, SocketAddr};

use axum::extract::RawQuery;
use axum::response::Redirect;
use axum::Json;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use tokio::net::UdpSocket;
use tracing::error;
use utoipa::ToSchema;

use crate::middleware::auth::jwt::{GameServerToken, WebUser};
use crate::res::{responses, Created};
use crate::{steam, Error, Result, State};

#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthRequest {
	api_key: u32,
	plugin_version: u16,
}

/// CS2 server authentication.
///
/// This endpoint is used by CS2 game servers to refresh their access token.
#[tracing::instrument(skip(state), fields(server_id, addr, token))]
#[utoipa::path(post, tag = "Auth", context_path = "/api", path = "/auth/refresh_token",
	request_body = AuthRequest,
	responses(
		responses::Ok<()>,
		responses::BadRequest,
		responses::Unauthorized,
		responses::InternalServerError,
	),
)]
pub async fn refresh_token(
	state: State,
	Json(AuthRequest { api_key, plugin_version }): Json<AuthRequest>,
) -> Result<Created<()>> {
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

	let server_info = GameServerToken::new(server.id, plugin_version);
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

	tracing::Span::current()
		.record("server_id", server.id)
		.record("addr", server_addr.to_string())
		.record("token", token);

	Ok(Created(()))
}

/// This is where the frontend will redirect users to when they click "login".
/// Steam will then redirect back to `steam_callback`, which will verify them.
#[tracing::instrument(skip_all)]
pub async fn steam_login(state: State) -> Redirect {
	state.steam().redirect()
}

#[tracing::instrument(skip_all, fields(steam_id))]
pub async fn steam_callback(
	state: State,
	RawQuery(query): RawQuery,
	mut cookies: CookieJar,
) -> Result<(CookieJar, Redirect)> {
	let query = query.ok_or(Error::Unauthorized)?;

	let user_data = serde_urlencoded::from_str::<steam::AuthResponse>(&query)
		.map(WebUser::from)
		.map_err(|_| Error::Unauthorized)?;

	tracing::Span::current().record("steam_id", user_data.steam_id.to_string());

	let auth_response = state
		.http_client()
		.post("https://steamcommunity.com/openid/login")
		.header(CONTENT_TYPE, "application/x-www-form-urlencoded")
		.body(query)
		.send()
		.await
		.and_then(|res| res.error_for_status());

	if auth_response.is_err() {
		return Err(Error::Unauthorized);
	}

	let jwt = state.jwt().encode(&user_data)?;
	let cookie = Cookie::build(("steam-id-token", jwt))
		.http_only(true)
		.secure(true)
		.permanent()
		.build();

	cookies = cookies.add(cookie);

	Ok((cookies, Redirect::to("https://cs2.kz")))
}
