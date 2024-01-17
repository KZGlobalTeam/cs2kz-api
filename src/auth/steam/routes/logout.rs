use axum::extract::Query;
use axum::response::Redirect;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::auth::Session;
use crate::extractors::{SessionToken, State};
use crate::{responses, steam, Result};

/// Query parameters for logging out.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct Logout {
	/// Redirect back to this URL once the logout process is complete.
	pub origin_url: Option<Url>,
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
    responses::SeeOther,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = []),
  ),
)]
pub async fn logout(
	state: State,
	session: Session,
	mut cookies: CookieJar,
	Query(Logout { origin_url }): Query<Logout>,
) -> Result<(CookieJar, Redirect)> {
	sqlx::query! {
		r#"
		UPDATE
		  WebSessions
		SET
		  expires_on = CURRENT_TIMESTAMP()
		WHERE
		  steam_id = ?
		"#,
		session.steam_id,
	}
	.execute(state.database())
	.await?;

	let config = state.config();

	cookies = remove_cookie(cookies, steam::Player::COOKIE_NAME, &config);
	cookies = remove_cookie(cookies, SessionToken::COOKIE_NAME, &config);

	let redirect_to = origin_url
		.as_ref()
		.map(Url::as_str)
		.unwrap_or(config.public_url.as_str());

	Ok((cookies, Redirect::to(redirect_to)))
}

fn remove_cookie(cookies: CookieJar, name: &'static str, config: &crate::Config) -> CookieJar {
	let mut cookie = Cookie::new(name, "");

	cookie.set_domain(config.public_url.host().unwrap().to_string());
	cookie.set_path("/");

	cookies.remove(cookie)
}
