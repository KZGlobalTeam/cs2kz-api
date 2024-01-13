use axum::extract::Query;
use axum::response::Redirect;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::extractors::State;
use crate::responses;

/// Query parameters for logging in with Steam.
#[derive(Debug, Deserialize, IntoParams)]
pub struct Login {
	/// The origin URL to redirect back to after the login process is complete.
	#[param(value_type = String)]
	pub origin_url: Url,
}

/// Log into Steam.
///
/// This route is used by websites.
#[tracing::instrument(skip(state))]
#[rustfmt::skip]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/steam/login",
  params(Login),
  responses(responses::SeeOther, responses::BadRequest),
)]
pub async fn login(state: State, Query(Login { origin_url }): Query<Login>) -> Redirect {
	state.steam_login().to_owned().with_origin_url(origin_url)
}
