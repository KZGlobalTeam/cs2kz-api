use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::course_sessions::{queries, CourseSession};
use crate::{responses, AppState, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Sessions",
  path = "/course-sessions/{session_id}",
  params(("session_id" = u64, Path, description = "The session's ID")),
  responses(
    responses::Ok<CourseSession>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(
	state: AppState,
	Path(session_id): Path<u64>,
) -> Result<Json<CourseSession>> {
	let mut query = QueryBuilder::new(queries::BASE_SELECT);

	query.push(" WHERE s.id = ").push_bind(session_id);

	let session = query
		.build_query_as::<CourseSession>()
		.fetch_optional(&state.database)
		.await?
		.ok_or(Error::no_data())?;

	Ok(Json(session))
}
