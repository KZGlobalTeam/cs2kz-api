//! Handlers for the `/bans` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use sqlx::encode::IsNull;
use time::OffsetDateTime;
use tracing::warn;
use utoipa::IntoParams;

use crate::auth::{Jwt, RoleFlags};
use crate::bans::{queries, Ban, BanReason, CreatedBan, NewBan};
use crate::parameters::{Limit, Offset};
use crate::responses::Created;
use crate::sqlx::{FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
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
		query.filter_is_null(" ub.id ", if unbanned { IsNull::No } else { IsNull::Yes });
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
	let (server, admin) = match (server, session) {
		(Some(server), None) => (Some(server.into_payload()), None),
		(None, Some(session)) => (None, Some(session.user())),
		(None, None) => {
			return Err(Error::unauthorized());
		}
		(Some(server), Some(session)) => {
			warn! {
				target: "audit_log",
				?server,
				?session,
				"request authenticated both as server and session",
			};

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
			.fetch_optional(&state.database)
			.await?
			.map(|row| row.ip_address)
			.ok_or_else(|| Error::unknown("player"))?,
	};

	let plugin_version_id = match server.map(|server| server.plugin_version_id().get()) {
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

	let expires_on = OffsetDateTime::now_utc() + reason.duration(previous_offenses);

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
		server.map(|server| server.id().get()),
		reason,
		admin.map(|admin| admin.steam_id()),
		plugin_version_id,
		expires_on,
	}
	.execute(&state.database)
	.await
	.map(crate::sqlx::last_insert_id)
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::unknown("player").with_source(err)
		} else if err.is_fk_violation_of("admin_id") {
			Error::unknown("admin").with_source(err)
		} else {
			Error::from(err)
		}
	})??;

	Ok(Created(Json(CreatedBan { ban_id })))
}
