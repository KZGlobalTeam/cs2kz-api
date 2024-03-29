//! Handlers for the `/records/{record_id}/replay` route.

use std::num::NonZeroU64;

use axum::extract::Path;
use axum::http::StatusCode;

use crate::{responses, AppState};

#[tracing::instrument(level = "debug", skip(_state))]
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
pub async fn get(_state: AppState, Path(_record_id): Path<NonZeroU64>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
