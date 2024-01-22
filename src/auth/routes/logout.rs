use axum::extract::Query;
use axum::response::Redirect;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::auth::Session;
use crate::extract::State;
use crate::{responses, Result};

#[derive(Debug, Deserialize, IntoParams)]
pub struct Logout {
	/// URL to redirect back to.
	pub return_to: Option<Url>,
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
	state: State,
	mut session: Session,
	Query(logout): Query<Logout>,
) -> Result<(Session, Redirect)> {
	session.invalidate(state.database()).await?;

	let config = state.config();
	let return_to = logout
		.return_to
		.as_ref()
		.map(Url::as_str)
		.unwrap_or(config.public_url.as_str());

	Ok((session, Redirect::to(return_to)))
}
