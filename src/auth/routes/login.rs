use axum::extract::Query;
use axum::response::Redirect;
use axum_extra::TypedHeader;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::auth::services::models::ServiceKey;
use crate::extract::State;
use crate::responses;

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
pub async fn login(
	state: State,
	TypedHeader(service_key): TypedHeader<ServiceKey>,
	Query(login): Query<Login>,
) -> Redirect {
	state
		.steam_login()
		.clone()
		.with_info(login.return_to, service_key)
}
