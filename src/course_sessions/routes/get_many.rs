use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::ServerIdentifier;
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::course_sessions::{queries, CourseSession};
use crate::database::ToID;
use crate::params::{Limit, Offset};
use crate::{query, responses, AppState, Error, Result};

#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct GetCourseSessionsParams<'a> {
	/// Filter by course.
	pub course_id: u32,

	/// Filter by a specific server.
	pub server: Option<ServerIdentifier<'a>>,

	/// Only include sessions after this timestamp.
	pub after: Option<DateTime<Utc>>,

	/// Only include sessions before this timestamp.
	pub before: Option<DateTime<Utc>>,

	/// Maximum amount of results.
	#[serde(default)]
	pub limit: Limit,

	/// Offset used for pagination.
	#[serde(default)]
	pub offset: Offset,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Sessions",
  path = "/course-sessions",
  params(GetCourseSessionsParams<'_>),
  responses(
    responses::Ok<CourseSession>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetCourseSessionsParams<'_>>,
) -> Result<Json<Vec<CourseSession>>> {
	let mut query = QueryBuilder::new(queries::BASE_SELECT);

	query
		.push(" WHERE s.course_id = ")
		.push_bind(params.course_id);

	if let Some(server) = params.server {
		let server_id = server.to_id(&state.database).await?;

		query.push(" AND sv.id = ").push_bind(server_id);
	}

	if let Some(after) = params.after {
		query.push(" AND s.created_on > ").push_bind(after);
	}

	if let Some(before) = params.before {
		query.push(" AND s.created_on < ").push_bind(before);
	}

	query.push(" ORDER BY s.id DESC ");
	query::push_limit(params.limit, params.offset, &mut query);

	let sessions = query
		.build_query_as::<CourseSession>()
		.fetch_all(&state.database)
		.await?;

	if sessions.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(sessions))
}
