//! Handlers for the `/records/{record_id}/replay` route.

use axum::extract::Path;
use axum::http::StatusCode;

use crate::responses;

#[tracing::instrument(level = "debug")]
#[utoipa::path(
  get,
  path = "/records/{record_id}/replay",
  tag = "Records",
  params(("record_id" = u64, Path, description = "The record's ID")),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(Path(_record_id): Path<u64>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
