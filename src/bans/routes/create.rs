use axum::{Extension, Json};
use chrono::{DateTime, Utc};

use crate::auth::{Jwt, Server, Session};
use crate::bans::{CreatedBan, NewBan};
use crate::extract::State;
use crate::responses::Created;
use crate::sqlx::SqlErrorExt;
use crate::{audit_error, responses, Error, Result};

/// Ban a player.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Bans",
  path = "/bans",
  responses(
    responses::Created<CreatedBan>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans"]),
    ("CS2 Server JWT" = []),
  ),
)]
pub async fn create(
	state: State,
	server: Option<Jwt<Server>>,
	session: Option<Extension<Session>>,
	Json(ban): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>> {
	if server.is_none() && session.is_none() {
		audit_error!(?ban, "ban submitted without authentication");
		return Err(Error::Unauthorized);
	}

	let (server_id, plugin_version_id) = server
		.map(|server| (server.id, server.plugin_version_id))
		.unzip();

	let banned_by = session.map(|session| session.user.steam_id);

	// FIXME(AlphaKeks): the ban duration should depend on the ban reason, and the reasons
	// are not mapped out as an enum yet
	let expires_on = None::<DateTime<Utc>>;

	let mut transaction = state.transaction().await?;

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
		.ok_or(Error::UnknownPlayer { steam_id: ban.steam_id })?,
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
		    banned_by,
		    expires_on
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?)
		"#,
		ban.steam_id,
		ip_address,
		ban.reason,
		server_id,
		plugin_version_id,
		banned_by,
		expires_on,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_foreign_key_violation_of("player_id") {
			Error::UnknownPlayer { steam_id: ban.steam_id }
		} else if err.is_foreign_key_violation_of("server_id") {
			panic!("unknown but authenticated server? {server_id:?} | {plugin_version_id:?}");
		} else if err.is_foreign_key_violation_of("plugin_version_id") {
			Error::InvalidPluginVersion { server_id: server_id.unwrap(), plugin_version_id }
		} else if err.is_foreign_key_violation_of("banned_by") {
			Error::UnknownPlayer { steam_id: banned_by.unwrap() }
		} else {
			Error::MySql(err)
		}
	})?;

	let ban_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

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
