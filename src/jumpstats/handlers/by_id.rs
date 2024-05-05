//! Handlers for the `/jumpstats/{jumpstat_id}` route.

use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::jumpstats::{queries, Jumpstat};
use crate::{responses, Error, Result, State};

/// Fetch a specific jumpstat by its ID.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/jumpstats/{jumpstat_id}",
  tag = "Jumpstats",
  params(("jumpstat_id" = u64, Path, description = "The jumpstat's ID")),
  responses(
    responses::Ok<Jumpstat>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: &State, Path(jumpstat_id): Path<u64>) -> Result<Json<Jumpstat>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE j.id = ").push_bind(jumpstat_id);

	let jumpstat = query
		.build_query_as::<Jumpstat>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::no_content())?;

	Ok(Json(jumpstat))
}
