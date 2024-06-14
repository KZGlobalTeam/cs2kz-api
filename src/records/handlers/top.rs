//! HTTP handlers for the `/records/top` routes.

use axum::extract::Query;
use axum::http::StatusCode;

use super::root::GetParams;
use crate::openapi::responses;
use crate::records::Record;

/// Fetch PB records.
#[tracing::instrument]
#[utoipa::path(
  get,
  path = "/records/top",
  tag = "Records",
  params(GetParams),
  responses(
    responses::Ok<Record>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(Query(_params): Query<GetParams>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
