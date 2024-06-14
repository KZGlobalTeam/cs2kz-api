//! HTTP handlers for the `/jumpstats/{jumpstat_id}/replay` routes.

use axum::extract::Path;
use axum::http::StatusCode;

use crate::jumpstats::JumpstatID;
use crate::openapi::responses;

/// Fetch a jumpstat replay.
#[tracing::instrument]
#[utoipa::path(
  get,
  path = "/jumpstats/{jumpstat_id}/replay",
  tag = "Jumpstats",
  params(("jumpstat_id" = u64, Path, description = "The jumpstat's ID")),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(Path(_jumpstat_id): Path<JumpstatID>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
