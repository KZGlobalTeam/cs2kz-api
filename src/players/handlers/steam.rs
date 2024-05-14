//! Handlers for the `/players/{player}/steam` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;

use crate::auth::SteamUser;
use crate::openapi::responses;
use crate::sqlx::FetchID;
use crate::{Result, State};

/// Fetch Steam profile information about a specific player.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}/steam",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<SteamUser>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: &State, Path(player): Path<PlayerIdentifier>) -> Result<Json<SteamUser>> {
	let steam_id = player.fetch_id(&state.database).await?;
	let user = SteamUser::fetch(steam_id, &state.http_client, &state.config).await?;

	Ok(Json(user))
}
