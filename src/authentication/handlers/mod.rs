//! HTTP handlers for the `/auth` routes.

use std::net::SocketAddr;

use authentication::Session;
use axum::extract::{ConnectInfo, Query};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::openapi::responses;
use crate::{authentication, steam, Result, State};

/// Query parameters for the login endpoint.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LoginParams {
	/// URL to redirect the user back to after a successful login.
	redirect_to: Url,
}

/// Login with Steam.
///
/// This will redirect the user to Steam, where they can login. A session for them will be created
/// and they're redirected back to `redirect_to`.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/login",
  tag = "Auth",
  params(LoginParams),
  responses(//
    responses::SeeOther,
    responses::BadRequest,
  ),
)]
pub async fn login(
	state: State,
	Query(LoginParams { redirect_to }): Query<LoginParams>,
) -> Redirect {
	authentication::steam::LoginForm::new(state.config.public_url.clone()).redirect_to(&redirect_to)
}

/// Query parameters for the logout endpoint.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct LogoutParams {
	/// Whether to invalidate all (still valid) sessions of this user.
	#[serde(default)]
	invalidate_all_sessions: bool,
}

/// Logout again.
///
/// This will invalidate your current session, and potentially every other session as well (if
/// `invalidate_all_sessions=true` is specified).
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/logout",
  tag = "Auth",
  security(("Browser Session" = [])),
  params(LogoutParams),
  responses(
    responses::SeeOther,
    responses::BadRequest,
    responses::Unauthorized,
  ),
)]
pub async fn logout(
	state: State,
	mut session: Session,
	cookies: CookieJar,
	Query(LogoutParams {
		invalidate_all_sessions,
	}): Query<LogoutParams>,
) -> Result<(Session, CookieJar, StatusCode)> {
	let mut transaction = state.transaction().await?;

	session
		.invalidate(invalidate_all_sessions, &mut transaction)
		.await?;

	transaction.commit().await?;

	tracing::debug!("user logged out");

	let user_cookie = Cookie::build((steam::user::COOKIE_NAME, ""))
		.domain(state.config.cookie_domain.clone())
		.path("/")
		.secure(cfg!(feature = "production"))
		.http_only(false)
		.expires(time::OffsetDateTime::now_utc())
		.build();

	Ok((session, cookies.add(user_cookie), StatusCode::OK))
}

/// The endpoint hit by Steam after a successful login.
///
/// This should not be used directly, and trying to do so will lead to errors.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/callback",
  tag = "Auth",
  params(authentication::steam::LoginResponse),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn callback(
	state: State,
	req_addr: ConnectInfo<SocketAddr>,
	cookies: CookieJar,
	login: authentication::steam::LoginResponse,
	user: steam::User,
) -> Result<(CookieJar, Redirect)> {
	let transaction = state.transaction().await?;
	let session = Session::create(&user, req_addr.ip(), &state.config, transaction).await?;
	let user_cookie = user.to_cookie(&state.config);
	let cookies = cookies.add(session).add(user_cookie);
	let redirect = Redirect::to(login.redirect_to.as_str());

	tracing::debug!("user logged in");

	Ok((cookies, redirect))
}
