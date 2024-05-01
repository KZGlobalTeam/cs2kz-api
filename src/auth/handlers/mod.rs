//! Handlers for the `/auth` routes.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use tracing::debug;
use url::Url;
use utoipa::IntoParams;

use super::{Session, SteamLoginResponse, SteamUser};
use crate::auth::SteamLoginForm;
use crate::{responses, Result, State};

/// Query parameters for logging in with Steam.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LoginParams {
	/// Where the user wants to be redirected to after the login process is done.
	redirect_to: Url,
}

#[tracing::instrument(level = "debug", skip(state))]
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
	state: &'static State,
	Query(LoginParams { redirect_to }): Query<LoginParams>,
) -> Redirect {
	SteamLoginForm::new(state.config.public_url.clone()).redirect_to(&redirect_to)
}

/// Query parameters for logging out.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LogoutParams {
	/// Whether *all* previous sessions should be invalidated.
	#[serde(default)]
	invalidate_all_sessions: bool,
}

#[tracing::instrument(level = "debug", skip(state))]
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
	state: &'static State,
	mut session: Session,
	Query(LogoutParams { invalidate_all_sessions }): Query<LogoutParams>,
) -> Result<(Session, StatusCode)> {
	let mut transaction = state.transaction().await?;

	session
		.invalidate(invalidate_all_sessions, &mut transaction)
		.await?;

	transaction.commit().await?;

	debug!(steam_id = %session.user().steam_id(), "user logged out");

	Ok((session, StatusCode::OK))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/auth/callback",
  tag = "Auth",
  params(SteamLoginResponse),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn callback(
	state: &'static State,
	cookies: CookieJar,
	login: SteamLoginResponse,
	user: SteamUser,
) -> Result<(CookieJar, Redirect)> {
	let transaction = state.transaction().await?;
	let session = Session::create(user.steam_id, &state.config, transaction).await?;
	let user_cookie = user.to_cookie(&state.config);
	let cookies = cookies.add(session).add(user_cookie);
	let redirect = Redirect::to(login.redirect_to.as_str());

	Ok((cookies, redirect))
}
