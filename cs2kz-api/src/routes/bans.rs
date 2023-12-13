//! This module holds all HTTP handlers related to bans.

use axum::extract::Query;
use axum::routing::get;
use axum::{Extension, Json, Router};
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder};
use utoipa::{IntoParams, ToSchema};

use crate::jwt::ServerClaims;
use crate::models::Ban;
use crate::responses::Created;
use crate::{openapi as R, sql, AppState, Error, Result, State};

/// This function returns the router for the `/bans` routes.
pub fn router(state: &'static AppState) -> Router {
	// let verify_gameserver =
	// 	|| axum::middleware::from_fn_with_state(state, crate::middleware::auth::verify_gameserver);

	Router::new()
		.route("/", get(get_bans))
		// .route("/", post(create_ban).layer(verify_gameserver()))
		.with_state(state)
}

/// This endpoint allows you to fetch bans.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Bans",
	path = "/bans",
	params(GetBansParams),
	responses(
		R::Ok<Ban>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_bans(
	state: State,
	Query(params): Query<GetBansParams<'_>>,
) -> Result<Json<Vec<Ban>>> {
	let mut query = QueryBuilder::new(
		r#"
		SELECT
			b.id,
			p1.steam_id player_id,
			p1.name player_name,
			b.reason,
			s.id server_id,
			s.name server_name,
			p2.steam_id banned_by_steam_id,
			p2.name banned_by_name,
			b.created_on,
			b.expires_on
		FROM
			Bans b
			JOIN Players p1 ON p1.steam_id = b.player_id
			LEFT JOIN Servers s ON s.id = b.server_id
			LEFT JOIN Players p2 ON p2.steam_id = b.banned_by
		"#,
	);

	let mut filter = sql::Filter::new();

	if let Some(player) = params.player {
		let steam_id = sql::fetch_steam_id(&player, state.database()).await?;

		query
			.push(filter)
			.push(" p1.steam_id = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(server) = params.server {
		let server_id = sql::fetch_server_id(&server, state.database()).await?;

		query.push(filter).push(" s.id = ").push_bind(server_id);

		filter.switch();
	}

	if let Some(reason) = params.reason {
		query.push(filter).push(" b.reason = ").push_bind(reason);

		filter.switch();
	}

	if let Some(banned_by) = params.banned_by {
		let steam_id = sql::fetch_steam_id(&banned_by, state.database()).await?;

		query
			.push(filter)
			.push(" p2.steam_id = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(expired) = params.expired {
		query
			.push(filter)
			.push(" b.expires_on ")
			.push(if expired { "<" } else { ">" })
			.push(" CURRENT_TIMESTAMP() ");

		filter.switch();
	}

	sql::push_limits::<500>(params.limit, params.offset, &mut query);

	let bans = query
		.build_query_as::<Ban>()
		.fetch_all(state.database())
		.await?;

	if bans.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(bans))
}

/// This endpoint is used by servers to ban players.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Bans",
	path = "/bans",
	security(("GameServer JWT" = [])),
	request_body = CreateBanRequest,
	responses(
		R::Created<CreatedBanResponse>,
		R::BadRequest,
		R::Unauthorized,
		R::Conflict,
		R::InternalServerError,
	),
)]
pub async fn create_ban(
	state: State,
	Extension(server): Extension<ServerClaims>,
	Json(body): Json<()>,
) -> Result<Created<Json<CreatedBanResponse>>> {
	todo!("figure out how to do this properly");
}

/// Query parameters for retrieving bans.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetBansParams<'a> {
	player: Option<PlayerIdentifier<'a>>,
	server: Option<ServerIdentifier<'a>>,
	reason: Option<String>,
	banned_by: Option<PlayerIdentifier<'a>>,
	expired: Option<bool>,

	#[param(minimum = 0, maximum = 500)]
	limit: Option<u64>,
	offset: Option<i64>,
}

/// A new ban.
#[derive(Debug, Serialize, FromRow, ToSchema)]
#[schema(example = json!({ "id": 69420 }))]
pub struct CreatedBanResponse {
	/// The ban's ID.
	pub id: u64,
}
