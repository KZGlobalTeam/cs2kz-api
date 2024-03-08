use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{ServerIdentifier, SteamID};
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::database::ToID;
use crate::params::{Limit, Offset};
use crate::players::CourseSession;
use crate::{query, AppState, Error, Result};

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

pub async fn get_course_sessions(
	state: AppState,
	Path(steam_id): Path<SteamID>,
	Query(params): Query<GetCourseSessionsParams<'_>>,
) -> Result<Json<Vec<CourseSession>>> {
	let mut query = QueryBuilder::new(
		r#"
		SELECT
		  s.id,
		  p.steam_id,
		  p.name player_name,
		  s.mode_id mode,
		  c.id course_id,
		  c.name course_name,
		  m.id map_id,
		  m.name map_name,
		  sv.id server_id,
		  sv.name server_name,
		  s.playtime,
		  s.total_runs,
		  s.finished_runs,
		  s.perfs,
		  s.bhops_tick0,
		  s.bhops_tick1,
		  s.bhops_tick2,
		  s.bhops_tick3,
		  s.bhops_tick4,
		  s.bhops_tick5,
		  s.bhops_tick6,
		  s.bhops_tick7,
		  s.bhops_tick8,
		  s.created_on
		FROM
		  CourseSessions s
		  JOIN Players p ON p.steam_id = s.player_id
		  JOIN Courses c ON c.id = s.course_id
		  JOIN Maps m ON m.id = c.map_id
		  JOIN Servers sv ON sv.id = s.server_id
		WHERE
		  p.steam_id =
		"#,
	);

	query
		.push_bind(steam_id)
		.push(" AND s.course_id = ")
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
