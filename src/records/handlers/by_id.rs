//! Handlers for the `/records/{record_id}` route.

use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::openapi::responses;
use crate::records::{queries, Record, RecordID};
use crate::{Error, Result, State};

/// Fetch a specific record by its ID.
#[tracing::instrument(skip(state))]
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
pub async fn get(state: State, Path(record_id): Path<RecordID>) -> Result<Json<Record>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE r.id = ").push_bind(record_id);

	let record = query
		.build_query_as::<Record>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::no_content())?;

	Ok(Json(record))
}
