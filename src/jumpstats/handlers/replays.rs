//! Handlers for the `/jumpstats/{jumpstat_id}/replay` route.

use axum::extract::Path;
use axum::http::StatusCode;

use crate::responses;

#[tracing::instrument(level = "debug")]
#[utoipa::path(
  get,
  path = "/jumpstats/{jumpstat_id}/replay",
  tag = "Jumpstats",
  params(("jumpstat_id" = u64, Path, description = "The jumpstat's ID")),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(Path(_jumpstat_id): Path<u64>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
