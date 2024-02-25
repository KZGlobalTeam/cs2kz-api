use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use tracing::trace;

use crate::auth::steam::Auth as SteamAuth;
use crate::auth::Session;
use crate::steam::Player;
use crate::{responses, AppState, Result};

/// This endpoint will be used by Steam during the login process (see `GET /auth/login`).
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
    responses::InternalServerError,
  ),
)]
pub async fn callback(
	state: AppState,
	auth: SteamAuth,
	cookies: CookieJar,
) -> Result<(CookieJar, Redirect)> {
	let steam_id = auth.steam_id();
	let player = Player::fetch(steam_id, &state.config.steam.api_key, &state.http_client).await?;

	trace!(?player, "fetched player from steam");

	let session = Session::new(steam_id, &state.database, &state.config).await?;
	let cookies = cookies.add(player.to_cookie(&state.config)).add(session);

	Ok((cookies, Redirect::to(auth.origin_url.as_str())))
}
