use axum::extract::Query;
use axum::Json;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::params::{Limit, Offset};
use crate::players::{FullPlayer, Player};
use crate::{responses, AppState, Error, Result};

/// Query Parameters for fetching [`Player`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetPlayersParams {
	/// Maximum amount of results.
	#[param(value_type = Option<u64>, maximum = 1000)]
	pub limit: Limit,

	/// Offset used for pagination.
	#[param(value_type = Option<i64>)]
	pub offset: Offset,
}

/// Fetch players who have joined a KZ server before.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players",
  params(GetPlayersParams),
  responses(
    responses::Ok<Player>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetPlayersParams>,
) -> Result<Json<Vec<FullPlayer>>> {
	let players = sqlx::query_as! {
		FullPlayer,
		r#"
		SELECT
		  p.steam_id `steam_id: SteamID`,
		  p.name,
		  (
		    SELECT
		      COUNT(b.id)
		    FROM
		      Bans b
		    WHERE
		      b.player_id = p.steam_id
		      AND b.expires_on > NOW()
		  ) `is_banned!: bool`
		FROM
		  Players p
		LIMIT
		  ? OFFSET ?
		"#,
		params.limit,
		params.offset,
	}
	.fetch_all(&state.database)
	.await?;

	if players.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(players))
}
