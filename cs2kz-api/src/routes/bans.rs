//! This module holds all HTTP handlers related to bans.

use std::net::Ipv4Addr;

use axum::extract::{Path, Query};
use axum::routing::{get, patch, post};
use axum::{Extension, Json, Router};
use chrono::{DateTime, Utc};
use cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder};
use tracing::error;
use utoipa::{IntoParams, ToSchema};

use crate::jwt::ServerClaims;
use crate::middleware::auth::web::Admin;
use crate::models::Ban;
use crate::permissions::Permissions;
use crate::responses::Created;
use crate::{openapi as R, sql, AppState, Error, Result, State};

/// This function returns the router for the `/bans` routes.
pub fn router(state: &'static AppState) -> Router {
	let add_ban = axum::middleware::from_fn_with_state(
		state,
		crate::middleware::auth::verify_game_server_or_web_user::<{ Permissions::BANS_ADD.0 }>,
	);

	let edit_ban = axum::middleware::from_fn_with_state(
		state,
		crate::middleware::auth::verify_web_user::<{ Permissions::BANS_EDIT.0 }>,
	);

	Router::new()
		.route("/", get(get_bans))
		.route("/", post(create_ban).layer(add_ban))
		.route("/:id", patch(update_ban).layer(edit_ban))
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

/// This endpoint is used by servers and `https://dashboard.cs2.kz` to ban players.
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
		R::InternalServerError,
	),
)]
pub async fn create_ban(
	state: State,
	server: Option<Extension<ServerClaims>>,
	admin: Option<Extension<Admin>>,
	Json(body): Json<CreateBanRequest>,
) -> Result<Created<Json<CreatedBanResponse>>> {
	if server.is_none() && admin.is_none() {
		error!("`POST /bans` has been hit without authentication");
		return Err(Error::Unauthorized);
	}

	let plugin_version = match server {
		Some(ref server) => server.plugin_version.to_string(),
		None => {
			sqlx::query! {
				r#"
				SELECT
					version
				FROM
					PluginVersions
				WHERE
					id = (
						SELECT
							MAX(id)
						FROM
							PluginVersions
					)
				"#,
			}
			.fetch_one(state.database())
			.await?
			.version
		}
	};

	let player_ip = match body.ip_address {
		Some(addr) => addr.to_string(),
		None => {
			sqlx::query!(
				r#"
				SELECT
					last_known_ip_address
				FROM
					Players
				WHERE
					steam_id = ?
				"#,
				body.steam_id,
			)
			.fetch_optional(state.database())
			.await?
			.ok_or(Error::UnknownPlayer { steam_id: body.steam_id })?
			.last_known_ip_address
		}
	};

	let mut transaction = state.begin_transaction().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Bans (
				player_id,
				player_ip,
				reason,
				server_id,
				plugin_version,
				banned_by,
				expires_on
			)
		VALUES
			(
				?,
				?,
				?,
				?,
				?,
				?,
				?
			)
		"#,
		body.steam_id.as_u32(),
		player_ip,
		body.reason,
		server.map(|claims| claims.id),
		plugin_version.to_string(),
		admin.map(|admin| admin.steam_id.as_u32()),
		body.expires_on,
	}
	.execute(transaction.as_mut())
	.await?;

	let id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await?
		.id;

	Ok(Created(Json(CreatedBanResponse { id })))
}

/// This endpoint is used by `https://dashboard.cs2.kz` to update bans
#[tracing::instrument]
#[utoipa::path(
	patch,
	tag = "Bans",
	path = "/bans/{id}",
	params(("id", Path, description = "The ID of the ban you wish to update.")),
	request_body = UpdateBanRequest,
	responses(
		R::Ok<()>,
		R::BadRequest,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn update_ban(
	state: State,
	Path(ban_id): Path<u32>,
	Json(body): Json<UpdateBanRequest>,
) -> Result<()> {
	let mut query = QueryBuilder::new("UPDATE Bans");
	let mut delimiter = " SET ";

	if let Some(reason) = body.reason {
		query.push(delimiter).push(" reason = ").push_bind(reason);

		delimiter = ",";
	}

	if let Some(expires_on) = body.expires_on {
		query
			.push(delimiter)
			.push(" expires_on = ")
			.push_bind(expires_on);
	}

	query.build().execute(state.database()).await?;

	Ok(())
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

/// Request body for a new ban submission.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CreateBanRequest {
	/// The player's SteamID.
	steam_id: SteamID,

	/// The reason for the ban.
	reason: String,

	/// The player's current IP address.
	#[param(value_type = Option<String>)]
	ip_address: Option<Ipv4Addr>,

	/// The expiration date for the ban.
	expires_on: Option<DateTime<Utc>>,
}

/// Request body for an update to a ban.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UpdateBanRequest {
	/// The reason for the ban.
	reason: Option<String>,

	/// The expiration date for the ban.
	expires_on: Option<DateTime<Utc>>,
}

/// A new ban.
#[derive(Debug, Serialize, FromRow, ToSchema)]
#[schema(example = json!({ "id": 69420 }))]
pub struct CreatedBanResponse {
	/// The ban's ID.
	pub id: u64,
}
