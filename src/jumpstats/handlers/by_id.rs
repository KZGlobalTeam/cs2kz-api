//! Handlers for the `/jumpstats/{jumpstat_id}` route.

use std::num::NonZeroU64;

use axum::extract::{Path, State};
use axum::Json;
use sqlx::{MySql, Pool, QueryBuilder};

use crate::jumpstats::{queries, Jumpstat};
use crate::{responses, Error, Result};

#[tracing::instrument(level = "debug", skip(database))]
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
pub async fn get(
	State(database): State<Pool<MySql>>,
	Path(jumpstat_id): Path<NonZeroU64>,
) -> Result<Json<Jumpstat>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE j.id = ").push_bind(jumpstat_id.get());

	let jumpstat = query
		.build_query_as::<Jumpstat>()
		.fetch_optional(&database)
		.await?
		.ok_or_else(|| Error::no_content())?;

	Ok(Json(jumpstat))
}
