//! Handlers for the `/records/top` route.

use axum::extract::Query;
use axum::http::StatusCode;

use super::root::GetParams;
use crate::records::Record;
use crate::{responses, AppState};

#[tracing::instrument(level = "debug", skip(_state))]
#[utoipa::path(
  get,
  path = "/records/top",
  tag = "Records",
  params(GetParams),
  responses(
    responses::Ok<Record>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(_state: AppState, Query(_params): Query<GetParams>) -> StatusCode {
	StatusCode::SERVICE_UNAVAILABLE
}
