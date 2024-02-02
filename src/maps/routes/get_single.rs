use axum::extract::{Path, Query};
use axum::Json;
use cs2kz::MapIdentifier;
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::database::GlobalStatus;
use crate::maps::{queries, KZMap};
use crate::{responses, AppState, Error, Result};

/// Query Parameters for fetching a [`KZMap`].
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct GetMapParams {
	/// Filter by global status.
	pub global_status: Option<GlobalStatus>,
}

/// Fetch a single map.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Maps",
  path = "/maps/{map}",
  params(MapIdentifier<'_>, GetMapParams),
  responses(
    responses::Ok<KZMap>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(
	state: AppState,
	Path(map): Path<MapIdentifier<'_>>,
	Query(params): Query<GetMapParams>,
) -> Result<Json<KZMap>> {
	let mut query = QueryBuilder::new(queries::BASE_SELECT);

	query.push(" WHERE ");

	match map {
		MapIdentifier::ID(id) => {
			query.push(" m.id = ").push_bind(id);
		}
		MapIdentifier::Name(name) => {
			query.push(" m.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	if let Some(global_status) = params.global_status {
		query
			.push(" AND m.global_status = ")
			.push_bind(global_status);
	}

	query.push(" ORDER BY m.id DESC ");

	query
		.build_query_as::<KZMap>()
		.fetch_all(&state.database)
		.await
		.map(KZMap::flatten)?
		.into_iter()
		.next()
		.map(Json)
		.ok_or(Error::no_data())
}
