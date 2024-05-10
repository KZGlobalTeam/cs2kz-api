//! Handlers for the `/records` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{Mode, PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use tracing::trace;
use utoipa::{IntoParams, ToSchema};

use crate::auth::Jwt;
use crate::kz::StyleFlags;
use crate::maps::FilterID;
use crate::parameters::{Limit, Offset, SortingOrder};
use crate::records::{queries, CreatedRecord, NewRecord, Record};
use crate::responses::{Created, PaginationResponse};
use crate::sqlx::{query, FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{auth, responses, Error, Result, State};

/// Query parameters for `GET /records`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by mode.
	mode: Option<Mode>,

	/// Filter by style(s).
	#[param(value_type = Vec<String>)]
	#[serde(default)]
	styles: StyleFlags,

	/// Filter by whether teleports where used.
	teleports: Option<bool>,

	/// Filter by player.
	player: Option<PlayerIdentifier>,

	/// Filter by server.
	server: Option<ServerIdentifier>,

	/// Filter by creation date.
	created_after: Option<DateTime<Utc>>,

	/// Filter by creation date.
	created_before: Option<DateTime<Utc>>,

	/// Sort by a specific property.
	///
	/// Defaults to "date".
	#[serde(default)]
	sort_by: SortRecordsBy,

	/// Decide the sorting order.
	///
	/// Defaults to "ascending".
	#[serde(default)]
	sort_order: SortingOrder,

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Sort records.
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
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/records",
  tag = "Records",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<Record>>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &State,
	Query(GetParams {
		mode,
		styles,
		teleports,
		player,
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

	Ok(Json(PaginationResponse { total, results: records }))
}

/// Create a new record.
///
/// This is used by CS2 servers to submit new records.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/records",
  tag = "Records",
  security(("CS2 Server" = [])),
  request_body = NewRecord,
  responses(
    responses::Created<CreatedRecord>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	state: &State,
	Jwt { payload: server, .. }: Jwt<auth::Server>,
	Json(NewRecord { player_id, mode, styles, course_id, teleports, time, bhop_stats }): Json<
		NewRecord,
	>,
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
	.ok_or_else(|| Error::unknown("course ID"))?;

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
		    perfs,
		    bhops_tick0,
		    bhops_tick1,
		    bhops_tick2,
		    bhops_tick3,
		    bhops_tick4,
		    bhops_tick5,
		    bhops_tick6,
		    bhops_tick7,
		    bhops_tick8,
		    legitimacy,
		    plugin_version_id
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?)
		"#,
		filter_id,
		styles.iter().copied().collect::<StyleFlags>(),
		teleports,
		time.as_secs_f64(),
		player_id,
		server.id(),
		bhop_stats.perfs,
		bhop_stats.tick0,
		bhop_stats.tick1,
		bhop_stats.tick2,
		bhop_stats.tick3,
		bhop_stats.tick4,
		bhop_stats.tick5,
		bhop_stats.tick6,
		bhop_stats.tick7,
		bhop_stats.tick8,
		server.plugin_version_id(),
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::unknown("player").with_source(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	transaction.commit().await?;

	trace!(%record_id, "inserted record");

	Ok(Created(Json(CreatedRecord { record_id })))
}
