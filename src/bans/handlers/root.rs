//! HTTP handlers for the `/bans` routes.

use std::net::{IpAddr, Ipv6Addr};

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use time::OffsetDateTime;
use utoipa::IntoParams;

use crate::authentication::Jwt;
use crate::authorization::Permissions;
use crate::bans::{queries, Ban, BanReason, CreatedBan, NewBan};
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses;
use crate::openapi::responses::{Created, PaginationResponse};
use crate::plugin::PluginVersionID;
use crate::sqlx::{query, FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{authentication, authorization, Error, Result, State};

/// Query parameters for `/bans`.
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

	/// Filter by admins who issued bans.
	banned_by: Option<PlayerIdentifier>,

	/// Filter by admins who reverted bans.
	unbanned_by: Option<PlayerIdentifier>,

	/// Only include bans submitted after this date.
	created_after: Option<DateTime<Utc>>,

	/// Only include bans submitted before this date.
	created_before: Option<DateTime<Utc>>,

	/// Maximum number of results to return.
	#[serde(default)]
	limit: Limit,

	/// Pagination offset.
	#[serde(default)]
	offset: Offset,
}

/// Fetch bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/bans",
  tag = "Bans",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<Ban>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: State,
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
		.fetch_all(transaction.as_mut())
		.await?;

	if bans.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: bans,
	}))
}

/// Create a new ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/bans",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  request_body = NewBan,
  responses(
    responses::Created<CreatedBan>,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn post(
	state: State,
	server: Option<Jwt<authentication::Server>>,
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	Json(NewBan {
		player_id,
		player_ip,
		reason,
	}): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>> {
	let (server, admin) = match (server, session) {
		(Some(server), None) => (Some(server.into_payload()), None),
		(None, Some(session)) => (None, Some(session.user())),
		(None, None) => {
			return Err(Error::unauthorized());
		}
		(Some(server), Some(session)) => {
			tracing::warn! {
				target: "cs2kz_api::audit_log",
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
	.ok_or_else(|| Error::not_found("player"))?;

	if already_banned {
		return Err(Error::already_exists("ban"));
	}

	let player_ip = match player_ip {
		Some(IpAddr::V4(ip)) => ip.to_ipv6_mapped(),
		Some(IpAddr::V6(ip)) => ip,
		None => sqlx::query_scalar! {
			r#"
			SELECT
			  ip_address `ip: Ipv6Addr`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			player_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or_else(|| Error::not_found("player"))?,
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
			Error::not_found("player").context(err)
		} else if err.is_fk_violation_of("admin_id") {
			Error::not_found("admin").context(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	transaction.commit().await?;

	tracing::trace! {
		target: "cs2kz_api::audit_log",
		%ban_id,
		%player_id,
		?reason,
		?server,
		?admin,
		"created ban",
	};

	Ok(Created(Json(CreatedBan { ban_id })))
}
