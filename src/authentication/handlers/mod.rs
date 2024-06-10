//! Handlers for the `/auth` routes.

use std::net::SocketAddr;

use authentication::Session;
use axum::extract::{ConnectInfo, Query};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::openapi::responses;
use crate::{authentication, steam, Result, State};

/// Query parameters for logging in with Steam.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LoginParams {
	/// Where the user wants to be redirected to after the login process is done.
	redirect_to: Url,
}

/// Log in with Steam.
///
/// This will redirect to Steam, and after you login, you will be sent back to `/auth/callback`,
/// and then to whatever the `redirect_to` query parameter is set to. A cookie with a session ID
/// will be inserted.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/login",
  tag = "Auth",
  params(LoginParams),
  responses(//
    responses::SeeOther,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn login(
	state: &State,
	Query(LoginParams { redirect_to }): Query<LoginParams>,
) -> Redirect {
	steam::authentication::LoginForm::new(state.config.public_url.clone()).redirect_to(&redirect_to)
}

/// Query parameters for logging out.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct LogoutParams {
	/// Whether *all* previous sessions should be invalidated.
	#[serde(default)]
	invalidate_all_sessions: bool,
}

/// Log out again.
///
/// This will invalidate your current session and delete your cookie.
/// If you wish to invalidate all other existing sessions as well, set
/// `invalidate_all_sessions=true`.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/logout",
  tag = "Auth",
  security(("Browser Session" = [])),
  params(LogoutParams),
  responses(//
    responses::SeeOther,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
)]
pub async fn logout(
	state: &State,
	mut session: Session,
	Query(LogoutParams {
		invalidate_all_sessions,
	}): Query<LogoutParams>,
) -> Result<(Session, StatusCode)> {
	let mut transaction = state.transaction().await?;

	session
		.invalidate(invalidate_all_sessions, &mut transaction)
		.await?;

	transaction.commit().await?;

	tracing::debug!("user logged out");

	Ok((session, StatusCode::OK))
}

/// The callback endpoint that will be hit by Steam after a successful login.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/callback",
  tag = "Auth",
  params(steam::authentication::LoginResponse),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn callback(
	state: &'static State,
	req_addr: ConnectInfo<SocketAddr>,
	cookies: CookieJar,
	login: steam::authentication::LoginResponse,
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
