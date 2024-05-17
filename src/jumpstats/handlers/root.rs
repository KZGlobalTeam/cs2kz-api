//! Handlers for the `/jumpstats` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{JumpType, Mode, PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use tracing::trace;
use utoipa::IntoParams;

use crate::authentication::{self, Jwt};
use crate::jumpstats::{queries, CreatedJumpstat, Jumpstat, NewJumpstat};
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses;
use crate::openapi::responses::{Created, PaginationResponse};
use crate::sqlx::{query, FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{Error, Result, State};

/// Query parameters for `GET /jumpstats`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by jump type.
	#[serde(rename = "type")]
	jump_type: Option<JumpType>,

	/// Filter by mode.
	mode: Option<Mode>,

	/// Filter by a minimum distance.
	minimum_distance: Option<f32>,

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

/// Fetch jumpstats.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/jumpstats",
  tag = "Jumpstats",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<Jumpstat>>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &State,
	Query(GetParams {
		jump_type,
		mode,
		minimum_distance,
		player,
		server,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<PaginationResponse<Jumpstat>>> {
	let mut query = FilteredQuery::new(queries::SELECT);
	let mut transaction = state.transaction().await?;

	if let Some(jump_type) = jump_type {
		query.filter(" j.type = ", jump_type);
	}

	if let Some(mode) = mode {
		query.filter(" j.mode_id = ", mode);
	}

	if let Some(minimum_distance) = minimum_distance {
		query.filter(" j.distance >= ", minimum_distance);
	}

	if let Some(player) = player {
		let steam_id = player.fetch_id(transaction.as_mut()).await?;

		query.filter(" j.player_id = ", steam_id);
	}

	if let Some(server) = server {
		let server_id = server.fetch_id(transaction.as_mut()).await?;

		query.filter(" j.server_id = ", server_id);
	}

	if let Some(created_after) = created_after {
		query.filter(" j.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter(" j.created_on < ", created_before);
	}

	query.push_limits(limit, offset);

	let jumpstats = query
		.build_query_as::<Jumpstat>()
		.fetch_all(transaction.as_mut())
		.await?;

	if jumpstats.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: jumpstats,
	}))
}

/// Create a new jumpstat.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/jumpstats",
  tag = "Jumpstats",
  security(("CS2 Server" = [])),
  request_body = NewJumpstat,
  responses(
    responses::Created<CreatedJumpstat>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	state: &State,
	Jwt {
		payload: server, ..
	}: Jwt<authentication::Server>,
	Json(NewJumpstat {
		jump_type,
		mode,
		player_id,
		strafes,
		distance,
		sync,
		pre,
		max,
		overlap,
		bad_angles,
		dead_air,
		height,
		airpath,
		deviation,
		average_width,
		airtime,
	}): Json<NewJumpstat>,
) -> Result<Created<Json<CreatedJumpstat>>> {
	let mut transaction = state.transaction().await?;

	let jumpstat_id = sqlx::query! {
		r#"
		INSERT INTO
		  Jumpstats (
		    type,
		    mode_id,
		    strafes,
		    distance,
		    sync,
		    pre,
		    max,
		    overlap,
		    bad_angles,
		    dead_air,
		    height,
		    airpath,
		    deviation,
		    average_width,
		    airtime,
		    player_id,
		    server_id,
		    plugin_version_id
		  )
		VALUES
		  (
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?
		  )
		"#,
		jump_type,
		mode,
		strafes,
		distance,
		sync,
		pre,
		max,
		overlap,
		bad_angles,
		dead_air,
		height,
		airpath,
		deviation,
		average_width,
		airtime.as_secs_f64(),
		player_id,
		server.id(),
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

	trace!(%jumpstat_id, "created jumpstat");

	Ok(Created(Json(CreatedJumpstat { jumpstat_id })))
}
