//! This module holds all HTTP handlers related to Steam authentication.

use axum::extract::RawQuery;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use tracing::trace;

use crate::jwt::UserClaims;
use crate::{steam, AppState, Error, Result, State};

static STEAM_LOGIN_VERIFY_URL: &str = "https://steamcommunity.com/openid/login";

/// This function returns the router for the `/auth/steam` routes.
pub fn router(state: &'static AppState) -> Router {
	Router::new()
		.route("/login", get(login))
		.route("/callback", get(callback))
		.with_state(state)
}

/// This is where the frontend will redirect users to when they click "login".
/// Steam will redirect them back to the API to confirm their identity.
#[tracing::instrument]
#[utoipa::path(get, tag = "Auth", path = "/auth/steam/login")]
pub async fn login(state: State) -> Redirect {
	state.steam_login()
}

/// This is where Steam will redirect a user back to after logging in.
/// We verify that the request actually comes from steam and give the user a cookie containing
/// a JWT.
#[tracing::instrument]
#[utoipa::path(get, tag = "Auth", path = "/auth/steam/callback")]
pub async fn callback(
	state: State,
	mut cookies: CookieJar,
	RawQuery(query): RawQuery,
) -> Result<(CookieJar, Redirect)> {
	let query = query.ok_or(Error::Unauthorized)?;
	let data = serde_urlencoded::from_str::<steam::AuthResponse>(&query)
		.map_err(|_| Error::Unauthorized)?;

	trace!(%data.steam_id, "user logged in with steam, verifying...");

	let validation = state
		.http_client()
		.post(STEAM_LOGIN_VERIFY_URL)
		.header("Content-Type", "application/x-www-form-urlencoded")
		.body(query)
		.send()
		.await
		.and_then(|res| res.error_for_status());

	if let Err(err) = validation {
		trace!(?err, "failed to authenticate user");

		return Err(Error::Unauthorized);
	}

	let claims = UserClaims::new(data.steam_id);
	let jwt = state.encode_jwt(&claims)?;
	let cookie = Cookie::build(("steam-id", jwt))
		.http_only(true)
		.secure(true)
		.permanent()
		.build();

	cookies = cookies.add(cookie);

	Ok((cookies, Redirect::to("https://cs2.kz")))
}
