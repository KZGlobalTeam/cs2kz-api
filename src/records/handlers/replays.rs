//! HTTP handlers for the `/records/{record_id}/replay` routes.

use axum::extract::Path;
use axum::http::StatusCode;

use crate::openapi::responses;
use crate::records::RecordID;

/// Fetch a record replay.
#[tracing::instrument]
#[utoipa::path(
  get,
  path = "/records/{record_id}/replay",
  tag = "Records",
  params(("record_id" = u64, Path, description = "The record's ID")),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(Path(_record_id): Path<RecordID>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
