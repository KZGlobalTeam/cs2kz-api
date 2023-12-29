//! This module holds all HTTP handlers related to jumpstats.

use axum::extract::{Path, Query};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use cs2kz::{Jumpstat, Mode, PlayerIdentifier, ServerIdentifier, SteamID, Style};
use serde::{Deserialize, Serialize};
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
			j.distance,
			j.mode_id,
			j.style_id,
			p.steam_id player_id,
			p.name player_name,
			s.id server_id,
			s.name server_name,
			j.created_on
		FROM
			Jumpstats j
			JOIN Players p ON p.steam_id = j.player_id
			JOIN Servers s ON s.id = j.server_id
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
	path = "/jumpstats/{steam_id}",
	security(("GameServer JWT" = [])),
	params(("steam_id" = SteamID, Path, description = "A player's SteamID.")),
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
	Path(steam_id): Path<SteamID>,
	Json(body): Json<CreateJumpstatRequest>,
) -> Result<Created<Json<CreatedJumpstatResponse>>> {
	let mut transaction = state.begin_transaction().await?;

	// TODO(AlphaKeks):
	//  1. figure out the criteria for jumpstats that should be saved / deleted
	//  2. make sure this jumpstat should be inserted
	//  3. check if any old jumpstats need to be deleted
	//  4. do all of the above in middleware (maybe)

	sqlx::query! {
		r#"
		INSERT INTO
			Jumpstats (
				`type`,
				distance,
				mode_id,
				style_id,
				player_id,
				server_id,
				plugin_version
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?)
		"#,
		body.kind,
		body.distance,
		body.mode,
		steam_id,
		body.style,
		server.id,
		server.plugin_version.to_string(),
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
	kind: Jumpstat,
	distance: f64,
	mode: Mode,
	style: Style,
}

/// A new jumpstat.
#[derive(Debug, Serialize, FromRow, ToSchema)]
#[schema(example = json!({ "id": 69420 }))]
pub struct CreatedJumpstatResponse {
	/// The jumpstat's ID.
	pub id: u64,
}
