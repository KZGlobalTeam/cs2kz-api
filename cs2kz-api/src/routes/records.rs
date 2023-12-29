//! This module holds all HTTP handlers related to records.

use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use cs2kz::{MapIdentifier, Mode, PlayerIdentifier, ServerIdentifier, SteamID, Style};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder};
use utoipa::{IntoParams, ToSchema};

use crate::jwt::ServerClaims;
use crate::models::{BhopStats, Record};
use crate::responses::Created;
use crate::sql::FetchID;
use crate::{openapi as R, sql, AppState, Error, Result, State};

/// This function returns the router for the `/records` routes.
pub fn router(state: &'static AppState) -> Router {
	let verify_gameserver =
		|| axum::middleware::from_fn_with_state(state, crate::middleware::auth::verify_gameserver);

	Router::new()
		.route("/", get(get_records))
		.route("/", post(create_record).layer(verify_gameserver()))
		.with_state(state)
}

/// This endpoint allows you to fetch records.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Records",
	path = "/records",
	params(GetRecordsParams),
	responses(
		R::Ok<Record>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_records(
	state: State,
	Query(params): Query<GetRecordsParams<'_>>,
) -> Result<Json<Vec<Record>>> {
	let mut query = QueryBuilder::new(
		r#"
		SELECT
			r.id,
			p.steam_id player_id,
			p.name player_name,
			f.course_id,
			m.id map_id,
			m.name map_name,
			c.map_stage,
			f.mode_id,
			r.style_id,
			f.tier,
			r.teleports,
			s.id server_id,
			s.name server_name,
			r.perfs,
			r.bhops_tick0,
			r.bhops_tick1,
			r.bhops_tick2,
			r.bhops_tick3,
			r.bhops_tick4,
			r.bhops_tick5,
			r.bhops_tick6,
			r.bhops_tick7,
			r.bhops_tick8,
			r.created_on
		FROM
			Records r
			JOIN Players p ON p.steam_id = r.player_id
			JOIN CourseFilters f on f.id = r.filter_id
			JOIN Courses c ON c.id = f.course_id
			JOIN Maps m ON m.id = c.map_id
			JOIN Servers s ON s.id = r.server_id
		"#,
	);

	let mut filter = sql::Filter::new();

	if let Some(player) = params.player {
		let steam_id = player.fetch_id(state.database()).await?;

		query
			.push(filter)
			.push(" r.player_id = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(map) = params.map {
		let map_id = map.fetch_id(state.database()).await?;

		query.push(filter).push(" m.id = ").push_bind(map_id);

		filter.switch();
	}

	if let Some(server) = params.server {
		let server_id = server.fetch_id(state.database()).await?;

		query.push(filter).push(" s.id = ").push_bind(server_id);

		filter.switch();
	}

	sql::push_limits::<100>(params.limit, params.offset, &mut query);

	let records = query
		.build_query_as::<Record>()
		.fetch_all(state.database())
		.await?;

	if records.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(records))
}

/// This endpoint is used by servers to send records.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Records",
	path = "/records",
	security(("GameServer JWT" = [])),
	request_body = CreateRecordRequest,
	responses(
		R::Created<CreatedRecordResponse>,
		R::BadRequest,
		R::Unauthorized,
		R::Conflict,
		R::InternalServerError,
	),
)]
pub async fn create_record(
	state: State,
	Extension(server): Extension<ServerClaims>,
	Json(body): Json<CreateRecordRequest>,
) -> Result<Created<Json<CreatedRecordResponse>>> {
	let mut transaction = state.begin_transaction().await?;

	let filter = sqlx::query! {
		r#"
		SELECT
			f.*
		FROM
			CourseFilters f
			JOIN Courses c ON c.id = f.course_id
			JOIN Maps m ON m.id = c.map_id
		WHERE
			m.id = ?
			AND c.map_stage = ?
			AND f.mode_id = ?
			AND f.has_teleports = ?
		"#,
		body.map_id,
		body.map_stage,
		body.mode,
		body.teleports > 0,
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.ok_or(Error::InvalidFilter)?;

	sqlx::query! {
		r#"
		INSERT INTO
			Records (
				player_id,
				filter_id,
				style_id,
				teleports,
				time,
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
				plugin_version
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		body.steam_id,
		filter.id,
		body.style,
		body.teleports,
		body.time,
		server.id,
		body.bhop_stats.perfs,
		body.bhop_stats.bhops_tick0,
		body.bhop_stats.bhops_tick1,
		body.bhop_stats.bhops_tick2,
		body.bhop_stats.bhops_tick3,
		body.bhop_stats.bhops_tick4,
		body.bhop_stats.bhops_tick5,
		body.bhop_stats.bhops_tick6,
		body.bhop_stats.bhops_tick7,
		body.bhop_stats.bhops_tick8,
		server.plugin_version.to_string(),
	}
	.execute(transaction.as_mut())
	.await?;

	let record_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await?
		.id;

	Ok(Created(Json(CreatedRecordResponse { record_id })))
}

/// Query parameters for retrieving records.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetRecordsParams<'a> {
	player: Option<PlayerIdentifier<'a>>,
	map: Option<MapIdentifier<'a>>,
	server: Option<ServerIdentifier<'a>>,

	#[param(minimum = 0, maximum = 500)]
	limit: Option<u64>,
	offset: Option<i64>,
}

/// A record.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "steam_id": "STEAM_1:1:161178172",
  "map_id": 1,
  "map_stage": 1,
  "mode": "kz_vanilla",
  "style": "normal",
  "teleports": 69,
  "time": 420.69,
  "bhop_stats": {
    "perfs": 200,
    "bhops_tick0": 100,
    "bhops_tick1": 100,
    "bhops_tick2": 30,
    "bhops_tick3": 10,
    "bhops_tick4": 10,
    "bhops_tick5": 0,
    "bhops_tick6": 0,
    "bhops_tick7": 0,
    "bhops_tick8": 0
  },
}))]
pub struct CreateRecordRequest {
	/// The SteamID of the player who set this record.
	steam_id: SteamID,

	/// The ID of the map the record was set on.
	map_id: u16,

	/// The stage the record was set on.
	map_stage: u8,

	/// The mode the record was set with.
	mode: Mode,

	/// The style the record was set with.
	style: Style,

	/// The amount of teleports used for setting this records.
	teleports: u32,

	/// The time taken to finish this run.
	time: f64,

	/// BunnyHop statistics about this run.
	bhop_stats: BhopStats,
}

/// A new record.
#[derive(Debug, Serialize, FromRow, ToSchema)]
#[schema(example = json!({ "record_id": 69420 }))]
pub struct CreatedRecordResponse {
	/// The record's ID.
	pub record_id: u64,
}
