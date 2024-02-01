use axum::extract::Path;
use axum::Json;

use crate::bans::{queries, Ban};
use crate::{responses, AppState, Error, Result};

/// Get a specific ban by ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Bans",
  path = "/bans/{ban_id}",
  params(("ban_id" = u32, Path, description = "The ban's ID")),
  responses(
    responses::Ok<Ban>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(state: AppState, Path(ban_id): Path<u32>) -> Result<Json<Ban>> {
	let query = format!("{} WHERE b.id = ?", queries::BASE_SELECT);

	sqlx::query_as(&query)
		.bind(ban_id)
		.fetch_optional(&state.database)
		.await?
		.map(Json)
		.ok_or(Error::no_data())
}
