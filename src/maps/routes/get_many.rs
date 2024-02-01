use axum::extract::Query;
use axum::Json;
use cs2kz::PlayerIdentifier;
use itertools::Itertools;
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::database::{GlobalStatus, ToID};
use crate::maps::{queries, KZMap};
use crate::params::{Limit, Offset};
use crate::query::Filter;
use crate::{responses, AppState, Error, Result};

/// Query Parameters for fetching [`KZMap`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetMapsParams<'a> {
	/// Filter by name.
	pub name: Option<String>,

	/// Filter by mapper.
	///
	/// This can be either a SteamID or name.
	pub mapper: Option<PlayerIdentifier<'a>>,

	/// Filter by [global status].
	///
	/// [global status]: GlobalStatus
	pub global_status: Option<GlobalStatus>,

	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

/// Officially approved KZ maps.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Maps",
  path = "/maps",
  params(GetMapsParams),
  responses(
    responses::Ok<KZMap>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetMapsParams<'_>>,
) -> Result<Json<Vec<KZMap>>> {
	let mut query = QueryBuilder::new(queries::BASE_SELECT);
	let mut filter = Filter::new();

	if let Some(ref name) = params.name {
		query
			.push(filter)
			.push(" m.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(ref player) = params.mapper {
		query.push(filter).push(
			r#"
			m.id IN (
			  SELECT
			    Maps.id
			  FROM
			    Maps
			    JOIN Mappers ON Mappers.map_id = Maps.id
			  WHERE
			    Mappers.player_id =
			"#,
		);

		let steam_id = player.to_id(&state.database).await?;

		query.push_bind(steam_id).push(")");
		filter.switch();
	}

	if let Some(global_status) = params.global_status {
		query
			.push(filter)
			.push(" m.global_status = ")
			.push_bind(global_status);

		filter.switch();
	}

	query.push(" ORDER BY m.id ASC ");

	let maps = query
		.build_query_as::<KZMap>()
		.fetch_all(&state.database)
		.await
		.map(KZMap::flatten)?
		.into_iter()
		.skip(params.offset.0 as _)
		.take(params.limit.0 as _)
		.collect_vec();

	if maps.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(maps))
}
