use axum::Json;
use serde_json::json;

use crate::auth::{Jwt, Role, Server, Session};
use crate::bans::{CreatedBan, NewBan};
use crate::responses::Created;
use crate::sqlx::SqlErrorExt;
use crate::{audit, responses, AppState, Error, Result};

/// Ban a player.
///
/// Requests with a SteamID of a player who is already banned will fail. Use `PATCH /bans/{ban_id}`
/// to update existing bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Bans",
  path = "/bans",
  responses(
    responses::Created<CreatedBan>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans"]),
    ("CS2 Server JWT" = []),
  ),
)]
pub async fn create(
	state: AppState,
	server: Option<Jwt<Server>>,
	session: Option<Session<{ Role::Bans as u32 }>>,
	Json(ban): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>> {
	if server.is_none() && session.is_none() {
		audit!(error, "ban submitted without authentication", ?ban);
		return Err(Error::bug());
	}

	let already_banned = sqlx::query! {
		r#"
		SELECT
		  is_banned `is_banned: bool`
		FROM
		  Players
		WHERE
		  steam_id = ?
		"#,
		ban.steam_id,
	}
	.fetch_optional(&state.database)
	.await?
	.map(|row| row.is_banned)
	.ok_or_else(|| Error::unknown("SteamID").with_detail(ban.steam_id))?;

	if already_banned {
		return Err(Error::already_exists("ban")
			.with_detail("player is currently banned; try to update their ban instead"));
	}

	let (server_id, plugin_version_id) = server
		.map(|server| (server.id, server.plugin_version_id))
		.unzip();

	let banned_by = session.map(|session| session.user.steam_id);

	let mut transaction = state.begin_transaction().await?;

	let ip_address = match ban.ip_address.map(|addr| addr.to_string()) {
		Some(addr) => addr,
		None => sqlx::query! {
			r#"
			SELECT
			  last_known_ip_address
			FROM
			  Players
			WHERE
			  steam_id = ?
			"#,
			ban.steam_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| row.last_known_ip_address)
		.ok_or_else(|| Error::unknown("SteamID").with_detail(ban.steam_id))?,
	};

	// If we didn't get a version from a server, just take the latest one.
	let plugin_version_id = match plugin_version_id {
		Some(id) => id,
		None => {
			sqlx::query!("SELECT MAX(id) `id!: u16` FROM PluginVersions")
				.fetch_one(transaction.as_mut())
				.await?
				.id
		}
	};

	sqlx::query! {
		r#"
		INSERT INTO
		  Bans (
		    player_id,
		    player_ip,
		    reason,
		    server_id,
		    plugin_version_id,
		    banned_by
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?)
		"#,
		ban.steam_id,
		ip_address,
		ban.reason,
		server_id,
		plugin_version_id,
		banned_by,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_foreign_key_violation_of("player_id") {
			Error::unknown("SteamID").with_detail(ban.steam_id)
		} else if err.is_foreign_key_violation_of("server_id") {
			Error::bug().with_detail(json!({
				"server_id": server_id,
				"plugin_version_id": plugin_version_id
			}))
		} else if err.is_foreign_key_violation_of("plugin_version_id") {
			Error::invalid("plugin version").with_detail(json!({
				"plugin_version_id": plugin_version_id
			}))
		} else if err.is_foreign_key_violation_of("banned_by") {
			Error::unknown("SteamID").with_detail(banned_by)
		} else {
			Error::from(err)
		}
	})?;

	let ban_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

	audit!("ban created", id = %ban_id, steam_id = %ban.steam_id, reason = %ban.reason);

	sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  is_banned = true
		WHERE
		  steam_id = ?
		"#,
		ban.steam_id,
	}
	.execute(transaction.as_mut())
	.await?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedBan { ban_id })))
}
