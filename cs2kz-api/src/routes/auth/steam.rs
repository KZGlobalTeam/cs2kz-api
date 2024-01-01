//! This module holds all HTTP handlers related to Steam authentication.

use axum::extract::Query;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use tracing::trace;
use url::{Host, Url};
use utoipa::IntoParams;

use crate::{openapi as R, steam, AppState, Error, Result, State};

/// This function returns the router for the `/auth/steam` routes.
pub fn router(state: &'static AppState) -> Router {
	Router::new()
		.route("/login", get(login))
		.route("/callback", get(callback))
		.with_state(state)
}

#[derive(Debug, Deserialize, IntoParams)]
struct LoginParams {
	#[param(value_type = String)]
	origin_url: Url,
}

/// This is where the frontend will redirect users to when they click "login".
/// Steam will redirect them back to the API to confirm their identity.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Auth",
	path = "/auth/steam/login",
	params(LoginParams),
	responses(R::Redirect, R::BadRequest)
)]
pub async fn login(state: State, Query(params): Query<LoginParams>) -> Redirect {
	state.steam_login(&params.origin_url)
}

/// This is where Steam will redirect a user back to after logging in.
/// We verify that the request actually comes from steam and give the user a cookie containing
/// a JWT.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Auth",
	path = "/auth/steam/callback",
	responses(R::Redirect, R::BadRequest, R::Unauthorized)
)]
pub async fn callback(
	state: State,
	mut cookies: CookieJar,
	Query(payload): Query<steam::AuthResponse>,
) -> Result<(CookieJar, Redirect)> {
	if payload.return_to.host() != state.public_url().host() {
		trace!(%payload.return_to, "invalid return URL");
		return Err(Error::Unauthorized);
	}

	let (steam_id, origin_url) = payload.validate(state.http_client()).await?;
	let host = origin_url.host().ok_or(Error::Unauthorized)?;
	let public_host = state.public_url().host().expect("we have a host");
	let is_known_host = match (&host, &public_host) {
		(Host::Ipv4(ip), Host::Ipv4(public_ip)) => ip == public_ip,
		(Host::Ipv6(ip), Host::Ipv6(public_ip)) => ip == public_ip,
		(Host::Domain(domain), Host::Domain(public_domain)) => {
			match domain.bytes().filter(|&b| b == b'.').count() {
				// cs2.kz
				1 => domain == public_domain,
				// dashboard.cs2.kz
				2 => domain.ends_with(public_domain),
				// ???
				_ => {
					trace!(%domain, "weird domain");
					false
				}
			}
		}

		_ => {
			trace!(%host, %public_host, "mismatching hosts");
			false
		}
	};

	if !is_known_host {
		trace!(%host, "unknown host");
		return Err(Error::Unauthorized);
	}

	// TODO: create a bunch of cookies depending on the user's permissions and add them to the
	// correct subdomains
	let cookie = Cookie::build(("steam_id", steam_id.to_string()))
		.http_only(true)
		.secure(true)
		.permanent() // TODO
		.build();

	cookies = cookies.add(cookie);

	Ok((cookies, Redirect::to(origin_url.as_str())))
}
