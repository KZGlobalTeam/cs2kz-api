//! This module holds all HTTP handlers related to Steam authentication.

use axum::extract::Query;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Transaction};
use time::OffsetDateTime;
use tracing::{trace, warn};
use url::{Host, Url};
use utoipa::IntoParams;

use crate::permissions::Permissions;
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
	let (steam_id, origin_url) = payload
		.validate(state.public_url(), state.http_client())
		.await?;

	let host = origin_url.host().ok_or_else(|| {
		trace!("origin URL did not have a host");
		Error::Unauthorized
	})?;

	let public_host = state.public_url().host().expect("we always have a host");
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

		_ if state.in_dev() => {
			warn!(%host, %public_host, "allowing invalid host due to dev mode");
			true
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

	let mut transaction = state.begin_transaction().await?;

	let user = sqlx::query!("SELECT * FROM Admins WHERE steam_id = ?", steam_id)
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or(Error::Unauthorized)?;

	let main = WebSession {
		steam_id,
		permissions: Permissions::GLOBAL_ADMIN & Permissions(user.permissions),
		subdomain: None,
	};

	let dashboard = WebSession {
		steam_id,
		permissions: Permissions::DASHBOARD & Permissions(user.permissions),
		subdomain: Some("dashboard"),
	};

	let main_token = main.create(&mut transaction).await?;
	let dashboard_token = dashboard.create(&mut transaction).await?;

	transaction.commit().await?;

	let host = host.to_string();
	let path = origin_url.path();
	let secure = state.in_prod();

	let main_cookie = main.cookie(&host, path, secure);
	let main_session_cookie = WebSession::session_cookie(main_token, &host, path, secure);

	let dashboard_host = format!("dashboard.{host}");
	let dashboard_cookie = dashboard.cookie(&dashboard_host, path, secure);
	let dashboard_session_cookie =
		WebSession::session_cookie(dashboard_token, dashboard_host, path, secure);

	cookies = cookies
		.add(main_cookie)
		.add(main_session_cookie)
		.add(dashboard_cookie)
		.add(dashboard_session_cookie);

	Ok((cookies, Redirect::to(origin_url.as_str())))
}

/// An authenticated session.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct WebSession {
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// The user's permissions.
	pub permissions: Permissions,

	#[serde(skip_serializing)]
	subdomain: Option<&'static str>,
}

impl WebSession {
	async fn create(self, transaction: &mut Transaction<'static, MySql>) -> Result<u64> {
		let token = 0;

		sqlx::query! {
			r#"
			INSERT INTO
			  WebSessions (subdomain, token, steam_id, expires_on)
			VALUES
			  (?, ?, ?, ?)
			"#,
			self.subdomain,
			token,
			self.steam_id,
			Self::expires_on(),
		}
		.execute(transaction.as_mut())
		.await?;

		Ok(token)
	}

	fn cookie(
		&self,
		host: impl Into<String>,
		path: impl Into<String>,
		secure: bool,
	) -> Cookie<'static> {
		let json = serde_json::to_string(self).expect("this is valid");

		Cookie::build(("kz-player", json))
			.domain(host.into())
			.path(path.into())
			.secure(secure)
			.expires(Self::expires_on())
			.build()
	}

	fn session_cookie(
		token: u64,
		host: impl Into<String>,
		path: impl Into<String>,
		secure: bool,
	) -> Cookie<'static> {
		Cookie::build(("kz-auth", token.to_string()))
			.domain(host.into())
			.path(path.into())
			.secure(secure)
			.http_only(true)
			.expires(Self::expires_on())
			.build()
	}

	fn expires_on() -> OffsetDateTime {
		OffsetDateTime::now_utc() + time::Duration::WEEK
	}
}
