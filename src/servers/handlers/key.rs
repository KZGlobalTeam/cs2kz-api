//! Handlers for the `/servers/key` route.

use std::time::Duration;

use axum::extract::Path;
use axum::Json;
use tracing::info;
use uuid::Uuid;

use crate::authentication::{self, Jwt};
use crate::authorization::Permissions;
use crate::openapi::responses::{self, Created, NoContent};
use crate::plugin::PluginVersionID;
use crate::servers::{RefreshKey, RefreshKeyRequest, RefreshKeyResponse, ServerID};
use crate::{authorization, Error, Result, State};

/// Generate a temporary authentication key for a CS2 server.
///
/// CS2 servers will use this endpoint together with their refresh key, to generate temporary
/// access keys, which will then be included in any following requests.
///
/// See `CS2 Servers` in `ARCHITECTURE.md` in the repository root for more details.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/servers/key",
  tag = "Servers",
  responses(
    responses::Created<Jwt<authentication::Server>>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn generate_temp(
	state: &State,
	Json(RefreshKeyRequest {
		refresh_key,
		plugin_version,
	}): Json<RefreshKeyRequest>,
) -> Result<Created<Json<RefreshKeyResponse>>> {
	let mut transaction = state.transaction().await?;

	let server = sqlx::query! {
		r#"
		SELECT
		  s.id `server_id: ServerID`,
		  v.id `plugin_version_id: PluginVersionID`
		FROM
		  Servers s
		  JOIN PluginVersions v ON v.semver = ?
		  AND s.refresh_key = ?
		"#,
		plugin_version.to_string(),
		refresh_key,
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.map(|row| authentication::Server::new(row.server_id, row.plugin_version_id))
	.ok_or_else(|| Error::invalid_cs2_refresh_key())?;

	let jwt = Jwt::new(&server, Duration::from_secs(60 * 15));
	let access_key = state
		.encode_jwt(jwt)
		.map(|access_key| RefreshKeyResponse { access_key })?;

	transaction.commit().await?;

	Ok(Created(Json(access_key)))
}

/// Generate a new refresh key for a server.
///
/// It will immediately invalidate the old **refresh** key, but cannot invalidate **access** keys,
/// as those are JWTs with set expiration dates.
///
/// This endpoint can be used by both admins and server owners.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  put,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(//
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
)]
pub async fn put_perma(
	state: &State,
	session: authentication::Session<authorization::IsServerAdminOrOwner>,
	Path(server_id): Path<ServerID>,
) -> Result<Created<Json<RefreshKey>>> {
	let mut transaction = state.transaction().await?;
	let refresh_key = Uuid::new_v4();
	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  refresh_key = ?
		WHERE
		  id = ?
		"#,
		refresh_key,
		server_id
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("server ID"));
	}

	transaction.commit().await?;

	info!(target: "audit_log", %server_id, %refresh_key, "generated new API key for server");

	Ok(Created(Json(RefreshKey { refresh_key })))
}

/// Delete a server's refresh key.
///
/// This can be used to effectively "de-global" server. Keep in mind though, that any previously
/// generated access keys are not invalidated, and will expire naturally.
///
/// This endpoint can only be hit by admins.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  delete,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(//
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
)]
pub async fn delete_perma(
	state: &State,
	session: authentication::Session<
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
	>,
	Path(server_id): Path<ServerID>,
) -> Result<NoContent> {
	let mut transaction = state.transaction().await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  refresh_key = NULL
		WHERE
		  id = ?
		"#,
		server_id,
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("server ID"));
	}

	transaction.commit().await?;

	info!(target: "audit_log", %server_id, "deleted API key for server");

	Ok(NoContent)
}

#[cfg(test)]
mod tests {
	use axum_extra::extract::cookie::Cookie;
	use cs2kz::SteamID;
	use reqwest::header;
	use uuid::Uuid;

	use crate::authentication;
	use crate::plugin::PluginVersionID;
	use crate::servers::{RefreshKey, RefreshKeyRequest, RefreshKeyResponse, ServerID};

	#[crate::integration_test]
	async fn generate_temp(ctx: &Context) {
		let server = sqlx::query! {
			r#"
			SELECT
			  s.id `id: ServerID`,
			  s.refresh_key `refresh_key!: uuid::fmt::Hyphenated`,
			  v.id `plugin_version_id: PluginVersionID`,
			  v.semver
			FROM
			  Servers s
			  JOIN PluginVersions v
			WHERE
			  s.id = 1
			LIMIT
			  1
			"#,
		}
		.fetch_one(&ctx.database)
		.await?;

		let refresh_key = RefreshKeyRequest {
			refresh_key: server.refresh_key.into(),
			plugin_version: server.semver.parse()?,
		};

		let response = ctx
			.http_client
			.post(ctx.url("/servers/key"))
			.json(&refresh_key)
			.send()
			.await?;

		assert_eq!(response.status(), 201);

		let RefreshKeyResponse { access_key } = response.json().await?;
		let server_info = ctx.decode_jwt::<authentication::Server>(&access_key)?;

		assert_eq!(server_info.id(), server.id);
		assert_eq!(server_info.plugin_version_id(), server.plugin_version_id);
	}

	#[crate::integration_test(fixtures = ["alphakeks-server-role"])]
	async fn put_perma(ctx: &Context) {
		let server = sqlx::query! {
			r#"
			SELECT
			  refresh_key `refresh_key!: uuid::fmt::Hyphenated`
			FROM
			  Servers
			WHERE
			  id = 1
			"#,
		}
		.fetch_one(&ctx.database)
		.await?;

		let response = ctx
			.http_client
			.put(ctx.url("/servers/1/key"))
			.send()
			.await?;

		assert_eq!(response.status(), 401);

		let alphakeks = SteamID::from_u64(76561198282622073_u64).unwrap();
		let session = ctx.auth_session(alphakeks).await?;
		let session_cookie = Cookie::from(session).encoded().to_string();

		let response = ctx
			.http_client
			.put(ctx.url("/servers/1/key"))
			.header(header::COOKIE, session_cookie)
			.send()
			.await?;

		assert_eq!(response.status(), 201);

		let RefreshKey { refresh_key } = response.json().await?;

		assert_ne!(refresh_key, Uuid::from(server.refresh_key));

		let server = sqlx::query! {
			r#"
			SELECT
			  refresh_key `refresh_key!: uuid::fmt::Hyphenated`
			FROM
			  Servers
			WHERE
			  id = 1
			"#,
		}
		.fetch_one(&ctx.database)
		.await?;

		assert_eq!(server.refresh_key, refresh_key.hyphenated());
	}

	#[crate::integration_test(fixtures = ["alphakeks-server-role"])]
	async fn delete_perma(ctx: &Context) {
		let server = sqlx::query! {
			r#"
			SELECT
			  refresh_key `refresh_key: uuid::fmt::Hyphenated`
			FROM
			  Servers
			WHERE
			  id = 1
			"#,
		}
		.fetch_one(&ctx.database)
		.await?;

		assert!(server.refresh_key.is_some());

		let response = ctx
			.http_client
			.delete(ctx.url("/servers/1/key"))
			.send()
			.await?;

		assert_eq!(response.status(), 401);

		let alphakeks = SteamID::from_u64(76561198282622073_u64).unwrap();
		let session = ctx.auth_session(alphakeks).await?;
		let session_cookie = Cookie::from(session).encoded().to_string();

		let response = ctx
			.http_client
			.delete(ctx.url("/servers/1/key"))
			.header(header::COOKIE, session_cookie)
			.send()
			.await?;

		assert_eq!(response.status(), 204);

		let server = sqlx::query! {
			r#"
			SELECT
			  refresh_key `refresh_key: uuid::fmt::Hyphenated`
			FROM
			  Servers
			WHERE
			  id = 1
			"#,
		}
		.fetch_one(&ctx.database)
		.await?;

		assert!(server.refresh_key.is_none());
	}
}
