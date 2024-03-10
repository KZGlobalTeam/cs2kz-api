use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{MapIdentifier, PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::database::ToID;
use crate::params::{Limit, Offset};
use crate::query::{self, FilteredQuery};
use crate::records::{queries, Record};
use crate::{responses, AppState, Error, Result};

#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetRecordsParams<'a> {
	/// Filter by a specific player.
	pub player: Option<PlayerIdentifier<'a>>,

	/// Filter by a specific map.
	pub map: Option<MapIdentifier<'a>>,

	/// Filter by a specific server.
	pub server: Option<ServerIdentifier<'a>>,

	/// Only include sessions after this timestamp.
	pub after: Option<DateTime<Utc>>,

	/// Only include sessions before this timestamp.
	pub before: Option<DateTime<Utc>>,

	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Records",
  path = "/records",
  params(GetRecordsParams<'_>),
  responses(
    responses::Ok<Record>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetRecordsParams<'_>>,
) -> Result<Json<Vec<Record>>> {
	let mut query = FilteredQuery::new(queries::BASE_SELECT);

	if let Some(player) = params.player {
		let steam_id = player.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" p.steam_id = ").push_bind(steam_id);
		});
	}

	if let Some(map) = params.map {
		let map_id = map.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" m.id = ").push_bind(map_id);
		});
	}

	if let Some(server) = params.server {
		let server_id = server.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" s.id = ").push_bind(server_id);
		});
	}

	if let Some(after) = params.after {
		query.filter(|query| {
			query.push(" r.created_on > ").push_bind(after);
		});
	}

	if let Some(before) = params.before {
		query.filter(|query| {
			query.push(" r.created_on < ").push_bind(before);
		});
	}

	query.push(" ORDER BY r.id DESC ");
	query::push_limit(params.limit, params.offset, &mut query);

	let records = query
		.build_query_as::<Record>()
		.fetch_all(&state.database)
		.await?;

	if records.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(records))
}
