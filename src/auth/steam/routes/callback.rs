use axum::extract::Query;
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use tracing::{trace, warn};

use crate::auth::steam::AuthRequest;
use crate::auth::Session;
use crate::extractors::State;
use crate::url::UrlExt;
use crate::{responses, steam, Error, Result};

/// Callback Route for Steam after a player logged in.
#[tracing::instrument(skip(state))]
#[rustfmt::skip]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/steam/callback",
  params(AuthRequest),
  responses(
    responses::SeeOther,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::Conflict,
    responses::InternalServerError,
  ),
)]
pub async fn callback(
	state: State,
	mut cookies: CookieJar,
	Query(auth): Query<AuthRequest>,
) -> Result<(CookieJar, Redirect)> {
	let config = state.config();
	let (steam_id, origin_url) = auth.validate(&config.public_url, state.http()).await?;

	let public_url = &config.public_url;
	let mut is_known_host = origin_url.host_eq_weak(public_url);

	if state.in_dev() && !is_known_host {
		warn!(%origin_url, %public_url, "allowing mismatching hosts due to dev mode");
		is_known_host = true;
	}

	if !is_known_host {
		trace!(%origin_url, %public_url, "rejecting unknown request origin");
		return Err(Error::Forbidden);
	}

	let player = steam::Player::get(steam_id, &config.steam.api_key, state.http()).await?;

	trace!(?player, "got player from steam");

	let origin_host = origin_url.host_str().ok_or_else(|| {
		trace!(%origin_url, "origin has no host");
		Error::Forbidden
	})?;

	let host = public_url.host_str().expect("API URL must have host");

	let subdomain = origin_url
		.subdomain()
		.map(str::parse)
		.transpose()
		.map_err(|_| Error::Unauthorized)?;

	let session =
		Session::create(steam_id, subdomain, host, state.in_prod(), state.database()).await?;

	cookies = cookies
		.add(player.cookie(origin_host, state.in_prod()))
		.add(session);

	let redirect = Redirect::to(origin_url.as_str());

	Ok((cookies, redirect))
}
