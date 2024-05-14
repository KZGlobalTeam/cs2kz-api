//! Handlers for the `/records/{record_id}/replay` route.

use axum::extract::Path;
use axum::http::StatusCode;

use crate::openapi::responses;
use crate::records::RecordID;

/// Fetch the replay file for a specific record.
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
pub async fn get(Path(_record_id): Path<RecordID>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
