//! Handlers for the `/bans` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::IntoParams;

use crate::auth::{Jwt, RoleFlags};
use crate::bans::{queries, Ban, BanReason, CreatedBan, NewBan};
use crate::parameters::{Limit, Offset};
use crate::responses::Created;
use crate::sqlx::{FetchID, FilteredQuery, QueryBuilderExt};
use crate::{auth, responses, AppState, Error, Result};

/// Query parameters for `GET /bans`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by player.
	player: Option<PlayerIdentifier>,

	/// Filter by server.
	server: Option<ServerIdentifier>,

	/// Filter by ban reason.
	reason: Option<BanReason>,

	/// Filter by bans that have already been reverted.
	unbanned: Option<bool>,

	/// Filter by admins responseible for bans.
	banned_by: Option<PlayerIdentifier>,

	/// Filter by admins responseible for unbans.
	unbanned_by: Option<PlayerIdentifier>,

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
  path = "/bans",
  tag = "Bans",
  params(GetParams),
  responses(
    responses::Ok<Ban>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: AppState,
	Query(GetParams {
		player,
		server,
		reason,
		unbanned,
		banned_by,
		unbanned_by,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<Vec<Ban>>> {
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(player) = player {
		let steam_id = player.fetch_id(&state.database).await?;

		query.filter(" b.player_id = ", steam_id);
	}

	if let Some(server) = server {
		let server_id = server.fetch_id(&state.database).await?;

		query.filter(" b.server_id = ", server_id);
	}

	if let Some(reason) = reason {
		query.filter(" b.reason = ", reason);
	}

	if let Some(unbanned) = unbanned {
		query.filter_is_null(" ub.id ", !unbanned);
	}

	if let Some(created_after) = created_after {
		query.filter(" b.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter(" b.created_on < ", created_before);
	}

	query.push_limits(limit, offset);

	let bans = query
		.build_query_as::<Ban>()
		.fetch_all(&state.database)
		.await?;

	if bans.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(bans))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/bans",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  request_body = NewBan,
  responses(
    responses::Created<CreatedBan>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	state: AppState,
	server: Option<Jwt<auth::Server>>,
	session: Option<auth::Session<auth::HasRoles<{ RoleFlags::BANS.as_u32() }>>>,
	Json(NewBan { player_id, player_ip, reason }): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>> {
	let (server, admin_id) = match (server, session) {
		(Some(server), None) => (Some((server.id(), server.plugin_version_id())), None),
		(None, Some(session)) => (None, Some(session.user().steam_id())),
		(None, None) | (Some(_), Some(_)) => {
			return Err(Error::unauthorized());
		}
	};

	let (already_banned, previous_offenses) = sqlx::query! {
		r#"
		SELECT
		  COUNT(b1.id) > 0 `already_banned: bool`,
		  COUNT(b2.id) `previous_bans: u8`
		FROM
		  Players p
		  LEFT JOIN Bans b1 ON b1.player_id = p.id
		  AND b1.expires_on > NOW()
		  LEFT JOIN Bans b2 ON b2.player_id = p.id
		  AND b2.expires_on < NOW()
		WHERE
		  p.id = ?
		"#,
		player_id,
	}
	.fetch_optional(&state.database)
	.await?
	.map(|row| (row.already_banned, row.previous_bans))
	.ok_or_else(|| Error::unknown("SteamID"))?;

	if already_banned {
		return Err(Error::already_exists("ban"));
	}

	let player_ip = match player_ip {
		Some(ip) => ip.to_string(),
		None => sqlx::query!("SELECT ip_address FROM Players WHERE id = ?", player_id)
			.fetch_one(&state.database)
			.await
			.map(|row| row.ip_address)?,
	};

	let plugin_version_id = match server.map(|(_, id)| id.get()) {
		Some(id) => id,
		None => sqlx::query! {
			r#"
			SELECT
			  id
			FROM
			  PluginVersions
			ORDER BY
			  created_on DESC
			LIMIT
			  1
			"#,
		}
		.fetch_one(&state.database)
		.await
		.map(|row| row.id)?,
	};

	let ban_id = sqlx::query! {
		r#"
		INSERT INTO
		  Bans (
		    player_id,
		    player_ip,
		    server_id,
		    reason,
		    admin_id,
		    plugin_version_id,
		    expires_on
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?)
		"#,
		player_id,
		player_ip,
		server.map(|(id, _)| id.get()),
		reason,
		admin_id,
		plugin_version_id,
		OffsetDateTime::now_utc() + reason.duration(previous_offenses),
	}
	.execute(&state.database)
	.await
	.map(crate::sqlx::last_insert_id)??;

	Ok(Created(Json(CreatedBan { ban_id })))
}
