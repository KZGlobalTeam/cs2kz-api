//! Handlers for the `/auth` routes.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use tracing::debug;
use url::Url;
use utoipa::IntoParams;

use super::{Session, SteamLoginResponse, SteamUser};
use crate::auth::SteamLoginForm;
use crate::sqlx::extract::{Connection, Transaction};
use crate::{responses, Result};

/// Query parameters for logging in with Steam.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LoginParams {
	/// Where the user wants to be redirected to after the login process is done.
	redirect_to: Url,
}

#[tracing::instrument(level = "debug", skip(config))]
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
	State(config): State<&'static crate::Config>,
	Query(LoginParams { redirect_to }): Query<LoginParams>,
) -> Redirect {
	SteamLoginForm::new(config.public_url.clone()).redirect_to(&redirect_to)
}

/// Query parameters for logging out.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LogoutParams {
	/// Whether *all* previous sessions should be invalidated.
	#[serde(default)]
	invalidate_all_sessions: bool,
}

#[tracing::instrument(level = "debug", skip(connection))]
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
	Connection(mut connection): Connection,
	mut session: Session,
	Query(LogoutParams { invalidate_all_sessions }): Query<LogoutParams>,
) -> Result<(Session, StatusCode)> {
	session
		.invalidate(invalidate_all_sessions, connection.as_mut())
		.await?;

	debug!(steam_id = %session.user().steam_id(), "user logged out");

	Ok((session, StatusCode::OK))
}

#[tracing::instrument(level = "debug", skip(config, transaction))]
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
	State(config): State<&'static crate::Config>,
	Transaction(transaction): Transaction,
	cookies: CookieJar,
	login: SteamLoginResponse,
	user: SteamUser,
) -> Result<(CookieJar, Redirect)> {
	let user_cookie = user.to_cookie(config);
	let session = Session::create(user.steam_id, config, transaction).await?;
	let cookies = cookies.add(user_cookie).add(session);
	let redirect = Redirect::to(login.redirect_to.as_str());

	Ok((cookies, redirect))
}
