//! Handlers for the `/records` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{Mode, PlayerIdentifier, ServerIdentifier, Style};
use serde::{Deserialize, Serialize};
use tracing::trace;
use utoipa::{IntoParams, ToSchema};

use crate::auth::Jwt;
use crate::parameters::{Limit, Offset};
use crate::records::{queries, CreatedRecord, NewRecord, Record, RecordID};
use crate::responses::Created;
use crate::sqlx::{FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{auth, responses, Error, Result, State};

/// Query parameters for `GET /records`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by mode.
	mode: Option<Mode>,

	/// Filter by style.
	style: Option<Style>,

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

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Response body for `GET /records`.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetResponse {
	/// The total amount of records available.
	///
	/// Used for pagination.
	total: u64,

	/// The records.
	records: Vec<Record>,
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/records",
  tag = "Records",
  params(GetParams),
  responses(
    responses::Ok<GetResponse>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &'static State,
	Query(GetParams {
		mode,
		style,
		teleports,
		player,
		server,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<GetResponse>> {
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(mode) = mode {
		query.filter(" f.mode_id = ", mode);
	}

	if let Some(_style) = style {
		// FIXME: flags
		// query.filter(" r.style_id = ", style);
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

	query.push_limits(limit, offset);

	let mut transaction = state.transaction().await?;

	let records = query
		.build_query_as::<Record>()
		.fetch_all(transaction.as_mut())
		.await?;

	let total = sqlx::query_scalar!("SELECT FOUND_ROWS() as total")
		.fetch_one(transaction.as_mut())
		.await?
		.try_into()
		.expect("how can a count be negative");

	transaction.commit().await?;

	if records.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(GetResponse { total, records }))
}

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
	state: &'static State,
	Jwt { payload: server, .. }: Jwt<auth::Server>,
	Json(NewRecord { player_id, mode, style, course_id, teleports, time, bhop_stats }): Json<
		NewRecord,
	>,
) -> Result<Created<Json<CreatedRecord>>> {
	let mut transaction = state.transaction().await?;

	let filter_id = sqlx::query! {
		r#"
		SELECT
		  id
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
	.map(|row| row.id)
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
		style,
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
	.last_insert_id();

	transaction.commit().await?;

	trace!(%record_id, "inserted record");

	Ok(Created(Json(CreatedRecord { record_id: RecordID(record_id) })))
}
