use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use tracing::{trace, warn};

use crate::auth::steam::Auth as SteamAuth;
use crate::auth::Session;
use crate::extract::State;
use crate::steam::Player;
use crate::url::UrlExt;
use crate::{responses, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/steam/callback",
  params(SteamAuth),
  responses( //
    responses::SeeOther,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::InternalServerError,
  ),
)]
pub async fn callback(
	state: State,
	auth: SteamAuth,
	cookies: CookieJar,
) -> Result<(CookieJar, Redirect)> {
	let config = state.config();
	let public_url = &config.public_url;
	let mut is_known_host = auth.origin_url.host_eq_weak(public_url);

	if !is_known_host && state.in_dev() {
		warn!(%auth.origin_url, %public_url, "allowing mismatching hosts due to dev mode");
		is_known_host = true;
	}

	if !is_known_host {
		trace!(%auth.origin_url, %public_url, "rejecting login due to mismatching hosts");
		return Err(Error::Forbidden);
	}

	let origin_host = auth.origin_url.host_str().ok_or_else(|| {
		trace!(%auth.origin_url, "origin somehow has no host");
		Error::Forbidden
	})?;

	let steam_id = auth.steam_id();
	let player = Player::fetch(steam_id, &config.steam.api_key, state.http()).await?;

	trace!(?player, "fetched player from steam");

	let mut transaction = state.transaction().await?;
	let session =
		Session::<0>::new(steam_id, &auth.origin_url, state.in_prod(), &mut transaction).await?;

	transaction.commit().await?;

	let cookies = cookies
		.add(player.to_cookie(origin_host, state.in_prod()))
		.add(session.cookie);

	Ok((cookies, Redirect::to(auth.origin_url.as_str())))
}
