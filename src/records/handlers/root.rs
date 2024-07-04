//! HTTP handlers for the `/records` routes.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{CourseIdentifier, MapIdentifier, Mode, PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::authentication::{self, Jwt};
use crate::kz::StyleFlags;
use crate::maps::FilterID;
use crate::openapi::parameters::{Limit, Offset, SortingOrder};
use crate::openapi::responses;
use crate::openapi::responses::{Created, PaginationResponse};
use crate::records::{queries, CreatedRecord, NewRecord, Record};
use crate::sqlx::{query, FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{Error, Result, State};

/// Query parameters for `/records`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by mode.
	mode: Option<Mode>,

	/// Filter by styles.
	#[param(value_type = Vec<String>)]
	#[serde(default)]
	styles: StyleFlags,

	/// Filter by whether teleports were used.
	teleports: Option<bool>,

	/// Filter by player.
	player: Option<PlayerIdentifier>,

	/// Filter by map.
	map: Option<MapIdentifier>,

	/// Filter by course.
	course: Option<CourseIdentifier>,

	/// Filter by server.
	server: Option<ServerIdentifier>,

	/// Only include records submitted after this date.
	created_after: Option<DateTime<Utc>>,

	/// Only include records submitted before this date.
	created_before: Option<DateTime<Utc>>,

	/// Which field to sort the results by.
	#[serde(default)]
	sort_by: SortRecordsBy,

	/// Which order to sort the results in.
	#[serde(default)]
	sort_order: SortingOrder,

	/// Maximum number of results to return.
	#[serde(default)]
	limit: Limit,

	/// Pagination offset.
	#[serde(default)]
	offset: Offset,
}

/// Fields to sort records by.
#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortRecordsBy {
	/// Sort by time.
	Time,

	/// Sort by date.
	#[default]
	Date,
}

/// Fetch records.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/records",
  tag = "Records",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<Record>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: State,
	Query(GetParams {
		mode,
		styles,
		teleports,
		player,
		map,
		course,
		server,
		created_after,
		created_before,
		sort_by,
		sort_order,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<PaginationResponse<Record>>> {
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(mode) = mode {
		query.filter(" f.mode_id = ", mode);
	}

	if styles != StyleFlags::NONE {
		query
			.filter(" ((r.style_flags & ", styles)
			.push(") = ")
			.push_bind(styles)
			.push(")");
	}

	match teleports {
		None => {}
		Some(true) => {
			query.filter(" r.teleports > ", 0);
		}
		Some(false) => {
			query.filter(" r.teleports = ", 0);
		}
	}

	if let Some(player) = player {
		let steam_id = player.fetch_id(&state.database).await?;

		query.filter(" r.player_id = ", steam_id);
	}

	if let Some(map) = map {
		let map_id = map.fetch_id(&state.database).await?;

		query.filter(" m.id = ", map_id);
	}

	if let Some(course) = course {
		let course_id = course.fetch_id(&state.database).await?;

		query.filter(" c.id = ", course_id);
	}

	if let Some(server) = server {
		let server_id = server.fetch_id(&state.database).await?;

		query.filter(" r.server_id = ", server_id);
	}

	if let Some(created_after) = created_after {
		query.filter(" r.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter(" r.created_on < ", created_before);
	}

	query.order_by(sort_order, match sort_by {
		SortRecordsBy::Time => "r.time",
		SortRecordsBy::Date => "r.created_on",
	});

	query.push_limits(limit, offset);

	let mut transaction = state.transaction().await?;

	let records = query
		.build_query_as::<Record>()
		.fetch_all(transaction.as_mut())
		.await?;

	if records.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: records,
	}))
}

/// Create a new record.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/records",
  tag = "Records",
  security(("CS2 Server" = [])),
  request_body = NewRecord,
  responses(
    responses::Created<CreatedRecord>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn post(
	state: State,
	Jwt {
		payload: server, ..
	}: Jwt<authentication::Server>,
	Json(NewRecord {
		player_id,
		mode,
		styles,
		course_id,
		teleports,
		time,
		bhop_stats,
	}): Json<NewRecord>,
) -> Result<Created<Json<CreatedRecord>>> {
	let mut transaction = state.transaction().await?;

	let filter_id = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  course_id = ?
		  AND mode_id = ?
		  AND teleports = ?
		"#,
		course_id,
		mode,
		teleports > 0,
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.ok_or_else(|| Error::not_found("course"))?;

	let record_id = sqlx::query! {
		r#"
		INSERT INTO
		  Records (
		    filter_id,
		    style_flags,
		    teleports,
		    time,
		    player_id,
		    server_id,
		    bhops,
		    perfs,
		    plugin_version_id
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		filter_id,
		styles.iter().copied().collect::<StyleFlags>(),
		teleports,
		time.as_secs_f64(),
		player_id,
		server.id(),
		bhop_stats.bhops,
		bhop_stats.perfs,
		server.plugin_version_id(),
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::not_found("player").context(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	transaction.commit().await?;

	tracing::trace!(%record_id, "created record");

	Ok(Created(Json(CreatedRecord { record_id })))
}
