//! HTTP handlers for this service.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::response::Redirect;
use axum::{routing, Router};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::{CookieJar, Query};
use serde::Deserialize;
use time::OffsetDateTime;

use super::{
	session,
	AuthService,
	LoginRequest,
	LoginResponse,
	LogoutRequest,
	LogoutResponse,
	Session,
};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::steam;

impl From<AuthService> for Router
{
	fn from(svc: AuthService) -> Self
	{
		let no_cors = Router::new()
			.route("/login", routing::get(login))
			.route("/callback", routing::get(callback))
			.with_state(svc.clone());

		let cors = Router::new()
			.route("/logout", routing::get(logout))
			.route("/verify-session", routing::get(verify_session))
			.route_layer(middleware::cors::dashboard([http::Method::OPTIONS, http::Method::GET]))
			.with_state(svc);

		no_cors.merge(cors)
	}
}

/// Login with Steam.
#[tracing::instrument(ret(level = "Debug"))]
#[utoipa::path(get, path = "/auth/login", tag = "Auth", params(LoginRequest))]
async fn login(State(svc): State<AuthService>, Query(req): Query<LoginRequest>) -> LoginResponse
{
	svc.login_url(req)
}

/// Query parameters for the [`logout`] handler.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct LogoutQuery
{
	/// Whether to invalidate all previous sessions, rather than just the
	/// current one.
	#[serde(default)]
	invalidate_all_sessions: bool,
}

/// Invalidate your existing session(s).
#[tracing::instrument(skip(cookies), err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/auth/logout", tag = "Auth", params(LogoutQuery))]
async fn logout(
	session: Session,
	State(svc): State<AuthService>,
	Query(LogoutQuery { invalidate_all_sessions }): Query<LogoutQuery>,
	cookies: CookieJar,
) -> Result<LogoutResponse, ProblemDetails>
{
	svc.logout(LogoutRequest { invalidate_all_sessions, session })
		.await?;

	let user_cookie = Cookie::build((steam::user::COOKIE_NAME, ""))
		.domain((*svc.cookie_domain).to_owned())
		.path("/")
		.secure(cfg!(feature = "production"))
		.same_site(SameSite::Lax)
		.http_only(false)
		.expires(OffsetDateTime::now_utc())
		.build();

	let session_cookie = Cookie::build((session::COOKIE_NAME, ""))
		.domain((*svc.cookie_domain).to_owned())
		.path("/")
		.secure(cfg!(feature = "production"))
		.same_site(SameSite::Lax)
		.http_only(true)
		.expires(OffsetDateTime::now_utc())
		.build();

	let cookies = cookies.add(user_cookie).add(session_cookie);

	Ok(LogoutResponse { cookies })
}

/// Verify whether a session token is still valid.
///
/// This endpoint will return a status in the 4xx range if you do not have a
/// valid session token, and a `200 OK` if you do.
#[tracing::instrument]
#[utoipa::path(get, path = "/auth/verify-session", tag = "Auth")]
async fn verify_session(session: Session)
{
	// We don't actually need to do anything here; the middleware will reject
	// any unauthorized requests, and `()` turns into an empty `200 OK`
	// response.
}

/// Hit by Steam after a successful login.
#[tracing::instrument(skip(cookies), err(Debug, level = "debug"))]
async fn callback(
	State(svc): State<AuthService>,
	ConnectInfo(req_addr): ConnectInfo<SocketAddr>,
	openid_payload: steam::OpenIDPayload,
	cookies: CookieJar,
) -> Result<(CookieJar, Redirect), ProblemDetails>
{
	let user = svc.steam_svc.fetch_user(openid_payload.steam_id()).await?;
	let user_cookie = user.to_cookie(&*svc.cookie_domain);
	let session_cookie = svc
		.login(user.steam_id, user.username, req_addr.ip().into())
		.await?
		.into_cookie(&*svc.cookie_domain);

	let cookies = cookies.add(user_cookie).add(session_cookie);
	let redirect = Redirect::to(openid_payload.redirect_to.as_str());

	Ok((cookies, redirect))
}
