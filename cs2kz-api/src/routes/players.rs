//! This module holds all HTTP handlers related to players.

use std::cmp;
use std::net::Ipv4Addr;
use std::time::Duration;

use axum::extract::{Path, Query};
use axum::routing::{get, patch, post};
use axum::{Extension, Json, Router};
use cs2kz::{PlayerIdentifier, SteamID};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::jwt::ServerClaims;
use crate::models::Player;
use crate::responses::Created;
use crate::{openapi as R, AppState, Error, Result, State};

/// This function returns the router for the `/players` routes.
pub fn router(state: &'static AppState) -> Router {
	let verify_gameserver =
		|| axum::middleware::from_fn_with_state(state, crate::middleware::auth::verify_gameserver);

	Router::new()
		.route("/", get(get_players))
		.route("/", post(create_player).layer(verify_gameserver()))
		.route("/:ident", get(get_player_by_ident))
		.route("/:ident", patch(update_player).layer(verify_gameserver()))
		.with_state(state)
}

/// This endpoint allows you to fetch players.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Players",
	path = "/players",
	params(GetPlayersParams),
	responses(
		R::Ok<Player>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_players(
	state: State,
	Query(params): Query<GetPlayersParams>,
) -> Result<Json<Vec<Player>>> {
	let limit = params.limit.map_or(100, |limit| cmp::min(limit, 500));
	let offset = params.offset.unwrap_or_default();

	let players = sqlx::query_as! {
		Player,
		r#"
		SELECT
			steam_id AS `steam_id: _`,
			name
		FROM
			Players
		LIMIT
			? OFFSET ?
		"#,
		limit,
		offset,
	}
	.fetch_all(state.database())
	.await?;

	if players.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(players))
}

/// Creates a new player.
///
/// Servers are expected to make a `GET` request for every joining player. If one of these requests
/// returns a `204` status code, the server should make a request to this endpoint to register the
/// player.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Players",
	path = "/players",
	security(("GameServer JWT" = [])),
	request_body = CreatePlayerRequest,
	responses(
		R::Created,
		R::BadRequest,
		R::Unauthorized,
		R::Conflict,
		R::InternalServerError,
	),
)]
pub async fn create_player(state: State, Json(body): Json<CreatePlayerRequest>) -> Result<Created> {
	sqlx::query! {
		r#"
		INSERT INTO
			Players (steam_id, name, last_known_ip_address)
		VALUES
			(?, ?, ?)
		"#,
		body.steam_id,
		body.name,
		body.ip_address.to_string(),
	}
	.execute(state.database())
	.await?;

	Ok(Created(()))
}

/// This endpoint allows you to fetch a single player by their SteamID or (parts of their) name.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Players",
	path = "/players/{ident}",
	params(("ident" = PlayerIdentifier<'_>, Path, description = "A player's SteamID or name.")),
	responses(
		R::Ok<Player>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_player_by_ident(
	state: State,
	Path(ident): Path<PlayerIdentifier<'_>>,
) -> Result<Json<Player>> {
	match ident {
		PlayerIdentifier::SteamID(steam_id) => {
			sqlx::query_as! {
				Player,
				r#"
				SELECT
					steam_id AS `steam_id: _`,
					name
				FROM
					Players
				WHERE
					steam_id = ?
				LIMIT
					1
				"#,
				steam_id,
			}
			.fetch_optional(state.database())
			.await?
		}
		PlayerIdentifier::Name(name) => {
			sqlx::query_as! {
				Player,
				r#"
				SELECT
					steam_id AS `steam_id: _`,
					name
				FROM
					Players
				WHERE
					name LIKE ?
				LIMIT
					1
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(state.database())
			.await?
		}
	}
	.ok_or(Error::NoContent)
	.map(Json)
}

/// Updates a player.
///
/// This route is reserved for CS2KZ servers!
///
/// Every time a map change happens, the server should make a request to this endpoint for every
/// player currently on the server.
///
/// Every time a player disconnects, the server should make a request to this endpoint for that
/// player.
#[tracing::instrument]
#[utoipa::path(
	patch,
	tag = "Players",
	path = "/players/{steam_id}",
	security(("GameServer JWT" = [])),
	params(("steam_id", Path, description = "The player's SteamID.")),
	request_body = UpdatePlayerRequest,
	responses(
		R::Ok,
		R::BadRequest,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn update_player(
	state: State,
	Extension(server): Extension<ServerClaims>,
	Path(steam_id): Path<SteamID>,
	Json(body): Json<UpdatePlayerRequest>,
) -> Result<()> {
	match (body.name, body.ip_address) {
		(None, None) => {}

		(Some(name), Some(ip_address)) => {
			sqlx::query! {
				r#"
				UPDATE
					Players
				SET
					name = ?,
					last_known_ip_address = ?
				WHERE
					steam_id = ?
				"#,
				name,
				ip_address.to_string(),
				steam_id,
			}
			.execute(state.database())
			.await?;
		}

		(Some(name), None) => {
			sqlx::query! {
				r#"
				UPDATE
					Players
				SET
					name = ?
				WHERE
					steam_id = ?
				"#,
				name,
				steam_id,
			}
			.execute(state.database())
			.await?;
		}

		(None, Some(ip_address)) => {
			sqlx::query! {
				r#"
				UPDATE
					Players
				SET
					last_known_ip_address = ?
				WHERE
					steam_id = ?
				"#,
				ip_address.to_string(),
				steam_id,
			}
			.execute(state.database())
			.await?;
		}
	}

	sqlx::query! {
		r#"
		INSERT INTO
			Sessions (
				player_id,
				server_id,
				time_active,
				time_spectating,
				time_afk,
				perfs,
				bhops_tick0,
				bhops_tick1,
				bhops_tick2,
				bhops_tick3,
				bhops_tick4,
				bhops_tick5,
				bhops_tick6,
				bhops_tick7,
				bhops_tick8
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id,
		server.id,
		body.session.time_active.as_secs(),
		body.session.time_spectating.as_secs(),
		body.session.time_afk.as_secs(),
		body.session.perfs,
		body.session.bhops_tick0,
		body.session.bhops_tick1,
		body.session.bhops_tick2,
		body.session.bhops_tick3,
		body.session.bhops_tick4,
		body.session.bhops_tick5,
		body.session.bhops_tick6,
		body.session.bhops_tick7,
		body.session.bhops_tick8,
	}
	.execute(state.database())
	.await?;

	Ok(())
}

/// Query parameters for retrieving information about players.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetPlayersParams {
	#[param(minimum = 0, maximum = 500)]
	limit: Option<u64>,
	offset: Option<i64>,
}

/// A new player.
///
/// This is expected to be sent by a CS2KZ server when a player joins for the first time.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "steam_id": "STEAM_1:1:161178172",
  "name": "AlphaKeks",
  "ip_address": "255.255.255.255"
}))]
pub struct CreatePlayerRequest {
	steam_id: SteamID,
	name: String,

	#[schema(value_type = String)]
	ip_address: Ipv4Addr,
}

/// An update to a player.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "name": "AlphaKeks",
  "ip_address": "255.255.255.255",
  "session": {
    "time_active": 600,
    "time_spectating": 51,
    "time_afk": 900,
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
  }
}))]
pub struct UpdatePlayerRequest {
	name: Option<String>,

	#[schema(value_type = Option<String>)]
	ip_address: Option<Ipv4Addr>,

	session: Session,
}

/// A player session.
///
/// This route is reserved for CS2KZ servers!
///
/// Anytime a player connects, a session is started. This session ends either when the map changes
/// or when the player disconnects.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "time_active": 600,
  "time_spectating": 51,
  "time_afk": 900,
  "perfs": 250,
  "bhops_tick0": 100,
  "bhops_tick1": 100,
  "bhops_tick2": 30,
  "bhops_tick3": 10,
  "bhops_tick4": 10,
  "bhops_tick5": 0,
  "bhops_tick6": 0,
  "bhops_tick7": 0,
  "bhops_tick8": 0
}))]
pub struct Session {
	#[serde(with = "crate::serde::duration_as_secs")]
	time_active: Duration,

	#[serde(with = "crate::serde::duration_as_secs")]
	time_spectating: Duration,

	#[serde(with = "crate::serde::duration_as_secs")]
	time_afk: Duration,

	perfs: u16,
	bhops_tick0: u16,
	bhops_tick1: u16,
	bhops_tick2: u16,
	bhops_tick3: u16,
	bhops_tick4: u16,
	bhops_tick5: u16,
	bhops_tick6: u16,
	bhops_tick7: u16,
	bhops_tick8: u16,
}
