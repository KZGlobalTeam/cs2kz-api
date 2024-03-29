//! Handlers for the `/jumpstats` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{JumpType, Mode, PlayerIdentifier, ServerIdentifier, Style};
use serde::Deserialize;
use tracing::trace;
use utoipa::IntoParams;

use crate::auth::Jwt;
use crate::jumpstats::{queries, CreatedJumpstat, Jumpstat, NewJumpstat};
use crate::parameters::{Limit, Offset};
use crate::responses::Created;
use crate::sqlx::{FetchID, FilteredQuery, QueryBuilderExt};
use crate::{auth, responses, AppState, Error, Result};

/// Query parameters for `GET /jumpstats`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by jump type.
	r#type: Option<JumpType>,

	/// Filter by mode.
	mode: Option<Mode>,

	/// Filter by style.
	style: Option<Style>,

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

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/jumpstats",
  tag = "Jumpstats",
  params(GetParams),
  responses(
    responses::Ok<Jumpstat>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: AppState,
	Query(GetParams {
		r#type,
		mode,
		style,
		minimum_distance,
		player,
		server,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<Vec<Jumpstat>>> {
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(r#type) = r#type {
		query.filter(" j.type = ", r#type);
	}

	if let Some(mode) = mode {
		query.filter(" j.mode_id = ", mode);
	}

	if let Some(style) = style {
		query.filter(" j.style_id = ", style);
	}

	if let Some(minimum_distance) = minimum_distance {
		query.filter(" j.distance >= ", minimum_distance);
	}

	if let Some(player) = player {
		let steam_id = player.fetch_id(&state.database).await?;

		query.filter(" j.player_id = ", steam_id);
	}

	if let Some(server) = server {
		let server_id = server.fetch_id(&state.database).await?;

		query.filter(" j.server_id = ", server_id);
	}

	if let Some(created_after) = created_after {
		query.filter(" j.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter(" j.created_on > ", created_before);
	}

	query.push_limits(limit, offset);

	let jumpstats = query
		.build_query_as::<Jumpstat>()
		.fetch_all(&state.database)
		.await?;

	if jumpstats.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(jumpstats))
}

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
	state: AppState,
	server: Jwt<auth::Server>,
	Json(NewJumpstat {
		r#type,
		mode,
		style,
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
	let jumpstat_id = sqlx::query! {
		r#"
		INSERT INTO
		  Jumpstats (
		    type,
		    mode_id,
		    style_id,
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
		    legitimacy,
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
		    ?,
		    0,
		    ?
		  )
		"#,
		r#type,
		mode,
		style,
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
		server.id().get(),
		server.plugin_version_id().get(),
	}
	.execute(&state.database)
	.await
	.map(crate::sqlx::last_insert_id)??;

	trace!(%jumpstat_id, "inserted jumpstat");

	Ok(Created(Json(CreatedJumpstat { jumpstat_id })))
}
