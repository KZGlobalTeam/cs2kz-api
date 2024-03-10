use axum::extract::Path;
use axum::Json;

use crate::query::FilteredQuery;
use crate::records::{queries, Record};
use crate::{responses, AppState, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Records",
  path = "/records/{record_id}",
  params(("record_id" = u64, Path, description = "The record's ID")),
  responses(
    responses::Ok<Record>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(state: AppState, Path(record_id): Path<u64>) -> Result<Json<Record>> {
	let mut query = FilteredQuery::new(queries::BASE_SELECT);

	query.push(" WHERE r.id = ").push_bind(record_id);

	let record = query
		.build_query_as::<Record>()
		.fetch_optional(&state.database)
		.await?
		.ok_or(Error::no_data())?;

	Ok(Json(record))
}
