use axum::extract::Query;
use axum::http::Uri;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::auth::Session;
use crate::extractors::{SessionToken, State};
use crate::url::UrlExt;
use crate::{responses, steam, Result};

/// Query parameters for logging out.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct Logout {
	/// Invalidate all existing sessions, not just the one for the current
	/// subdomain.
	#[serde(default)]
	pub invalidate_all_sessions: bool,
}

/// Logout, invalidating all existing sessions.
///
/// This route is used by websites.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/steam/logout",
  params(Logout),
  responses(
    responses::Ok<()>,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = []),
  ),
)]
pub async fn logout(
	state: State,
	cookies: CookieJar,
	session: Session,
	uri: Uri,
	Query(Logout { invalidate_all_sessions }): Query<Logout>,
) -> Result<CookieJar> {
	let mut query = QueryBuilder::new(
		r#"
		UPDATE
		  WebSessions
		SET
		  expires_on = CURRENT_TIMESTAMP()
		WHERE
		  steam_id =
		"#,
	);

	query.push_bind(session.steam_id).push(" AND (subdomain ");

	if let Some(subdomain) = uri.subdomain() {
		query.push(" = ").push_bind(subdomain);
	} else {
		query.push(" IS NULL ");
	}

	query.push(if invalidate_all_sessions {
		" OR TRUE) "
	} else {
		")"
	});

	tracing::debug!(query = %query.sql());

	query.build().execute(state.database()).await?;

	let mut cookies = cookies.remove(Cookie::from(SessionToken::COOKIE_NAME));

	// TODO(AlphaKeks): delete the cookies from the other subdomains as well
	if invalidate_all_sessions {
		cookies = cookies.remove(Cookie::from(steam::Player::COOKIE_NAME));
	}

	Ok(cookies)
}
