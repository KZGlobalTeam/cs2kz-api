use axum::extract::Query;
use axum::response::Redirect;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::{responses, AppState};

#[derive(Debug, Deserialize, IntoParams)]
pub struct Login {
	/// The URL to return to after logging in.
	pub return_to: Url,
}

/// Redirects the user to Steam to log in, and then back to `return_to` after a successful
/// login.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/login",
  params(Login),
  responses( //
    responses::SeeOther,
    responses::BadRequest,
  ),
)]
pub async fn login(state: AppState, Query(login): Query<Login>) -> Redirect {
	state.steam_login().clone().origin_url(login.return_to)
}
