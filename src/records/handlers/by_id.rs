//! Handlers for the `/records/{record_id}` route.

use std::num::NonZeroU64;

use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::records::{queries, Record};
use crate::{responses, AppState, Error, Result};

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/records/{record_id}",
  tag = "Records",
  params(("record_id" = u64, Path, description = "The record's ID")),
  responses(
    responses::Ok<Record>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: AppState, Path(record_id): Path<NonZeroU64>) -> Result<Json<Record>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE r.id = ").push_bind(record_id.get());

	let record = query
		.build_query_as::<Record>()
		.fetch_optional(&state.database)
		.await?
		.ok_or(Error::no_content())?;

	Ok(Json(record))
}
