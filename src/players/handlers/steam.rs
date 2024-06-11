//! Handlers for the `/players/{player}/steam` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;

use crate::openapi::responses;
use crate::sqlx::FetchID;
use crate::{steam, Result, State};

/// Fetch Steam profile information about a specific player.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}/steam",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<steam::User>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: State, Path(player): Path<PlayerIdentifier>) -> Result<Json<steam::User>> {
	let steam_id = player.fetch_id(&state.database).await?;
	let user = steam::User::fetch(steam_id, &state.http_client, &state.config).await?;

	Ok(Json(user))
}
