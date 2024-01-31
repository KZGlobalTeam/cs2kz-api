use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;
use sqlx::QueryBuilder;

use crate::players::Player;
use crate::{responses, AppState, Error, Result};

/// Get a specific player by SteamID or name.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players/{player}",
  params(PlayerIdentifier<'_>),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(
	state: AppState,
	Path(player): Path<PlayerIdentifier<'_>>,
) -> Result<Json<Player>> {
	let mut query = QueryBuilder::new("SELECT steam_id, name, is_banned FROM Players WHERE");

	match player {
		PlayerIdentifier::SteamID(steam_id) => {
			query.push(" steam_id = ").push_bind(steam_id);
		}
		PlayerIdentifier::Name(name) => {
			query.push(" name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	query
		.build_query_as::<Player>()
		.fetch_optional(state.database())
		.await
		.map_err(Error::from)?
		.map(Json)
		.ok_or(Error::no_data())
}
