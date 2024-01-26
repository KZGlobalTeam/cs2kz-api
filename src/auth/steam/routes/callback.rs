use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use tracing::trace;

use crate::auth::steam::Auth as SteamAuth;
use crate::auth::{Service, Session};
use crate::extract::State;
use crate::steam::Player;
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
	let mut transaction = state.transaction().await?;

	let steam_id = auth.steam_id();
	let service = Service::from_key(auth.service_key, transaction.as_mut()).await?;
	let player = Player::fetch(steam_id, &config.steam.api_key, state.http()).await?;

	trace!(?player, "fetched player from steam");

	let domain = auth.origin_url.host_str().ok_or_else(|| {
		trace!("origin has no host");
		Error::ForeignHost
	})?;

	let session =
		Session::<0>::new(service, steam_id, domain, state.in_prod(), &mut transaction).await?;

	transaction.commit().await?;

	let cookies = cookies
		.add(player.to_cookie(auth.origin_url.as_str(), state.in_prod()))
		.add(session.cookie);

	Ok((cookies, Redirect::to(auth.origin_url.as_str())))
}
