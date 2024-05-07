//! Handlers for the `/bans` route.

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use sqlx::encode::IsNull;
use time::OffsetDateTime;
use tracing::{trace, warn};
use utoipa::IntoParams;

use crate::auth::{Jwt, RoleFlags};
use crate::bans::{queries, Ban, BanReason, CreatedBan, NewBan};
use crate::parameters::{Limit, Offset};
use crate::plugin::PluginVersionID;
use crate::responses::{Created, PaginationResponse};
use crate::sqlx::{query, FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{auth, responses, Error, Result, State};

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

/// Fetch bans.
///
/// These are bans that might have expired / have been reverted. If that's the case, they will also
/// include the according "unban" entry.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/bans",
  tag = "Bans",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<Ban>>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &State,
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
) -> Result<Json<PaginationResponse<Ban>>> {
	let mut query = FilteredQuery::new(queries::SELECT);
	let mut transaction = state.transaction().await?;

	if let Some(player) = player {
		let steam_id = player.fetch_id(transaction.as_mut()).await?;

		query.filter(" b.player_id = ", steam_id);
	}

	if let Some(server) = server {
		let server_id = server.fetch_id(transaction.as_mut()).await?;

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
		.fetch_all(transaction.as_mut())
		.await?;

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	if bans.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(PaginationResponse { total, results: bans }))
}

/// Ban a player.
///
/// This endpoint can be used by both CS2 servers and admins.
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
	state: &State,
	server: Option<Jwt<auth::Server>>,
	session: Option<auth::Session<auth::HasRoles<{ RoleFlags::BANS.value() }>>>,
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

	let mut transaction = state.transaction().await?;

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
	.fetch_optional(transaction.as_mut())
	.await?
	.map(|row| (row.already_banned, row.previous_bans))
	.ok_or_else(|| Error::unknown("SteamID"))?;

	if already_banned {
		return Err(Error::already_exists("ban"));
	}

	let player_ip = match player_ip {
		Some(ip) => ip.to_string(),
		None => sqlx::query_scalar!("SELECT ip_address FROM Players WHERE id = ?", player_id)
			.fetch_optional(transaction.as_mut())
			.await?
			.ok_or_else(|| Error::unknown("player"))?,
	};

	let plugin_version_id = if let Some(id) = server.map(|server| server.plugin_version_id()) {
		id
	} else {
		sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: PluginVersionID`
			FROM
			  PluginVersions
			ORDER BY
			  created_on DESC
			LIMIT
			  1
			"#,
		}
		.fetch_one(transaction.as_mut())
		.await?
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
		server.map(|server| server.id()),
		reason,
		admin.map(|admin| admin.steam_id()),
		plugin_version_id,
		expires_on,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::unknown("player").with_source(err)
		} else if err.is_fk_violation_of("admin_id") {
			Error::unknown("admin").with_source(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	transaction.commit().await?;

	trace!(%ban_id, %player_id, ?reason, ?server, ?admin, "created ban");

	Ok(Created(Json(CreatedBan { ban_id })))
}
