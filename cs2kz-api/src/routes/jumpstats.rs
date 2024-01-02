//! This module holds all HTTP handlers related to jumpstats.

use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use cs2kz::{Jumpstat, Mode, PlayerIdentifier, ServerIdentifier, SteamID, Style};
use serde::{Deserialize, Serialize};
use sqlx::types::Decimal;
use sqlx::{FromRow, QueryBuilder};
use utoipa::{IntoParams, ToSchema};

use crate::jwt::ServerClaims;
use crate::models::JumpstatResponse;
use crate::responses::Created;
use crate::sql::FetchID;
use crate::{openapi as R, sql, AppState, Error, Result, State};

/// This function returns the router for the `/jumpstats` routes.
pub fn router(state: &'static AppState) -> Router {
	let verify_gameserver =
		|| axum::middleware::from_fn_with_state(state, crate::middleware::auth::verify_gameserver);

	Router::new()
		.route("/", get(get_jumpstats))
		.route("/:steam_id", post(create_jumpstat).layer(verify_gameserver()))
		.with_state(state)
}

/// This endpoint allows you to fetch jumpstats.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Jumpstats",
	path = "/jumpstats",
	params(GetJumpstatsParams),
	responses(
		R::Ok<JumpstatResponse>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_jumpstats(
	state: State,
	Query(params): Query<GetJumpstatsParams<'_>>,
) -> Result<Json<Vec<JumpstatResponse>>> {
	let mut query = QueryBuilder::new(
		r#"
		SELECT
			j.id,
			j.type kind,
			j.mode_id,
			j.style_id,
			j.strafes,
			j.distance,
			j.sync,
			j.pre,
			j.max,
			j.overlap,
			j.bad_air,
			j.dead_air,
			j.height,
			j.airpath,
			j.deviation,
			j.average_width,
			j.airtime,
			p.steam_id player_id,
			p.name player_name,
			s.id server_id,
			s.name server_name,
			v.version,
			j.created_on
		FROM
			Jumpstats j
			JOIN Players p ON p.steam_id = j.player_id
			JOIN Servers s ON s.id = j.server_id
			JOIN PluginVersions v ON v.id = j.plugin_version_id
		"#,
	);

	let mut filter = sql::Filter::new();

	if let Some(kind) = params.kind {
		query.push(filter).push(" j.type = ").push_bind(kind);

		filter.switch();
	}

	if let Some(distance) = params.minimum_distance {
		query
			.push(filter)
			.push(" j.distance >= ")
			.push_bind(distance);

		filter.switch();
	}

	if let Some(mode) = params.mode {
		query.push(filter).push(" j.mode_id = ").push_bind(mode);

		filter.switch();
	}

	if let Some(style) = params.style {
		query.push(filter).push(" j.style_id = ").push_bind(style);

		filter.switch();
	}

	if let Some(player) = params.player {
		let steam_id = player.fetch_id(state.database()).await?;

		query
			.push(filter)
			.push(" j.player_id = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(server) = params.server {
		let id = server.fetch_id(state.database()).await?;

		query.push(filter).push(" j.server_id = ").push_bind(id);

		filter.switch();
	}

	sql::push_limits::<500>(params.limit, params.offset, &mut query);

	let jumpstats = query
		.build_query_as::<JumpstatResponse>()
		.fetch_all(state.database())
		.await?;

	if jumpstats.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(jumpstats))
}

/// This endpoint is used by servers to send jumpstats.
///
/// Servers are supposed to fetch the player's PBs when they join and only make requests to this
/// route with new PBs, but the endpoint will still validate the jumpstat, so it is expected to
/// return a 409 occasionally.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Jumpstats",
	path = "/jumpstats",
	security(("GameServer JWT" = [])),
	request_body = CreateJumpstatRequest,
	responses(
		R::Created<CreatedJumpstatResponse>,
		R::BadRequest,
		R::Unauthorized,
		R::Conflict,
		R::InternalServerError,
	),
)]
pub async fn create_jumpstat(
	state: State,
	Extension(server): Extension<ServerClaims>,
	Json(body): Json<CreateJumpstatRequest>,
) -> Result<Created<Json<CreatedJumpstatResponse>>> {
	let pb = sqlx::query! {
		r#"
		SELECT
			distance
		FROM
			Jumpstats
		WHERE
			mode_id = ?
		ORDER BY
			distance DESC
		LIMIT
			1
		"#,
		body.mode,
	}
	.fetch_optional(state.database())
	.await?;

	if pb.is_some_and(|pb| pb.distance > body.distance) {
		return Err(Error::NotPersonalBest);
	}

	let mut transaction = state.begin_transaction().await?;

	sqlx::query! {
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
				bad_air,
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
			(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		body.kind,
		body.mode,
		body.style,
		body.strafes,
		body.distance,
		body.sync,
		body.pre,
		body.max,
		body.overlap,
		body.bad_air,
		body.dead_air,
		body.height,
		body.airpath,
		body.deviation,
		body.average_width,
		body.airtime,
		body.steam_id,
		server.id,
		server.plugin_version_id,
	}
	.execute(transaction.as_mut())
	.await?;

	let id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await?
		.id;

	Ok(Created(Json(CreatedJumpstatResponse { id })))
}

/// Query parameters for retrieving jumpstats.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetJumpstatsParams<'a> {
	kind: Option<Jumpstat>,
	minimum_distance: Option<f64>,
	mode: Option<Mode>,
	style: Option<Style>,
	player: Option<PlayerIdentifier<'a>>,
	server: Option<ServerIdentifier<'a>>,

	#[param(minimum = 0, maximum = 500)]
	limit: Option<u64>,
	offset: Option<i64>,
}

/// A jumpstat.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "kind": "longjump",
  "distance": 230.3418,
  "mode": "kz_vanilla",
  "style": "backwards"
}))]
pub struct CreateJumpstatRequest {
	steam_id: SteamID,
	kind: Jumpstat,
	mode: Mode,
	style: Style,
	strafes: u8,
	distance: Decimal,
	sync: Decimal,
	pre: Decimal,
	max: Decimal,
	overlap: Decimal,
	bad_air: Decimal,
	dead_air: Decimal,
	height: Decimal,
	airpath: Decimal,
	deviation: Decimal,
	average_width: Decimal,
	airtime: Decimal,
}

/// A new jumpstat.
#[derive(Debug, Serialize, FromRow, ToSchema)]
#[schema(example = json!({ "id": 69420 }))]
pub struct CreatedJumpstatResponse {
	/// The jumpstat's ID.
	pub id: u64,
}
