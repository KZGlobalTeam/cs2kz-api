use axum::extract::Query;
use axum::response::Redirect;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::auth::Session;
use crate::{responses, AppState, Result};

#[derive(Debug, Deserialize, IntoParams)]
pub struct Logout {
	/// URL to redirect back to.
	pub return_to: Option<Url>,

	/// Invalidate *all* old sessions.
	#[serde(default)]
	pub all: bool,
}

/// Invalidates the user's current session and clears out any cookies.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/logout",
  params(Logout),
  responses( //
    responses::SeeOther,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
)]
pub async fn logout(
	state: AppState,
	mut session: Session,
	Query(logout): Query<Logout>,
) -> Result<(Session, Redirect)> {
	session.invalidate(logout.all, &state.database).await?;

	let redirect = logout
		.return_to
		.as_ref()
		.map(|url| Redirect::to(url.as_str()))
		.unwrap_or_else(|| Redirect::to(state.config.public_url.as_str()));

	Ok((session, redirect))
}
