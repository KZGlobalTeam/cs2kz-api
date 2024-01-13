use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::extractors::State;
use crate::params::{Limit, Offset};
use crate::players::Player;
use crate::{responses, Error, Result};

/// Query Parameters for fetching [`Player`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetPlayersParams {
	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
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
	state: State,
	Query(params): Query<GetPlayersParams>,
) -> Result<Json<Vec<Player>>> {
	let players = sqlx::query_as(
		r#"
		SELECT
		  steam_id,
		  name,
		  is_banned
		FROM
		  Players
		LIMIT
		  ? OFFSET ?
		"#,
	)
	.bind(params.limit)
	.bind(params.offset)
	.fetch_all(state.database())
	.await
	.map_err(Error::from)?;

	if players.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(players))
}
