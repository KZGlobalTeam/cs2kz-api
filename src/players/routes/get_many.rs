use axum::extract::Query;
use axum::Json;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::params::{Limit, Offset};
use crate::players::Player;
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

/// Players who have joined officially approved KZ servers.
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
) -> Result<Json<Vec<Player>>> {
	let players = sqlx::query_as! {
		Player,
		r#"
		SELECT
		  steam_id `steam_id: SteamID`,
		  name,
		  is_banned `is_banned: bool`
		FROM
		  Players
		LIMIT
		  ? OFFSET ?
		"#,
		params.limit,
		params.offset,
	}
	.fetch_all(state.database())
	.await?;

	if players.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(players))
}
