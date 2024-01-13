use axum::extract::Path;
use axum::Json;
use cs2kz::MapIdentifier;
use sqlx::QueryBuilder;

use crate::extractors::State;
use crate::maps::{queries, KZMap};
use crate::{responses, Error, Result};

/// Fetch a single map by ID or name.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Maps",
  path = "/maps/{map}",
  params(MapIdentifier<'_>),
  responses(
    responses::Ok<KZMap>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(state: State, Path(map): Path<MapIdentifier<'_>>) -> Result<Json<KZMap>> {
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

	query
		.build_query_as::<KZMap>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.reduce(KZMap::reduce)
		.map(Json)
		.ok_or(Error::NoContent)
}
