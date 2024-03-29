//! Handlers for the `/servers/key` route.

use std::num::NonZeroU16;
use std::time::Duration;

use axum::extract::Path;
use axum::Json;
use tracing::info;
use uuid::Uuid;

use crate::auth::{Jwt, RoleFlags};
use crate::responses::{self, Created, NoContent};
use crate::servers::{RefreshKey, RefreshKeyRequest, RefreshKeyResponse};
use crate::{auth, AppState, Error, Result};

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/servers/key",
  tag = "Servers",
  responses(
    responses::Created<Jwt<auth::Server>>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn generate_temp(
	state: AppState,
	Json(RefreshKeyRequest { refresh_key, plugin_version }): Json<RefreshKeyRequest>,
) -> Result<Created<Json<RefreshKeyResponse>>> {
	let server = sqlx::query! {
		r#"
		SELECT
		  s.id server_id,
		  v.id plugin_version_id
		FROM
		  Servers s
		  JOIN PluginVersions v ON v.semver = ?
		  AND s.refresh_key = ?
		"#,
		plugin_version.to_string(),
		refresh_key,
	}
	.fetch_optional(&state.database)
	.await?
	.and_then(|row| {
		Some((
			NonZeroU16::new(row.server_id)?,
			NonZeroU16::new(row.plugin_version_id)?,
		))
	})
	.map(|(server_id, plugin_version_id)| auth::Server::new(server_id, plugin_version_id))
	.ok_or(Error::invalid_refresh_key())?;

	let access_key = state
		.encode_jwt(&server, Duration::from_secs(60 * 15))
		.map(|access_key| RefreshKeyResponse { access_key })?;

	Ok(Created(Json(access_key)))
}

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
	state: AppState,
	session: auth::Session<
		auth::Either<auth::HasRoles<{ RoleFlags::SERVERS.as_u32() }>, auth::IsServerOwner>,
	>,
	Path(server_id): Path<NonZeroU16>,
) -> Result<Created<Json<RefreshKey>>> {
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
		server_id.get()
	}
	.execute(&state.database)
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("server ID"));
	}

	info!(target: "audit_log", %server_id, %refresh_key, "generated new API key for server");

	Ok(Created(Json(RefreshKey { refresh_key })))
}

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
	state: AppState,
	session: auth::Session<auth::HasRoles<{ RoleFlags::SERVERS.as_u32() }>>,
	Path(server_id): Path<NonZeroU16>,
) -> Result<NoContent> {
	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  refresh_key = NULL
		WHERE
		  id = ?
		"#,
		server_id.get(),
	}
	.execute(&state.database)
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("server ID"));
	}

	info!(target: "audit_log", %server_id, "deleted API key for server");

	Ok(NoContent)
}

#[cfg(test)]
mod tests {
	use axum_extra::extract::cookie::Cookie;
	use cs2kz::SteamID;
	use reqwest::header;
	use uuid::Uuid;

	use crate::auth;
	use crate::servers::{RefreshKey, RefreshKeyRequest, RefreshKeyResponse};

	#[crate::test]
	async fn generate_temp(ctx: &Context) {
		let server = sqlx::query! {
			r#"
			SELECT
			  s.id,
			  s.refresh_key `refresh_key!: uuid::fmt::Hyphenated`,
			  v.id plugin_version_id,
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
		let server_info = ctx.decode_jwt::<auth::Server>(&access_key)?;

		assert_eq!(server_info.id().get(), server.id);
		assert_eq!(server_info.plugin_version_id().get(), server.plugin_version_id);
	}

	#[crate::test(fixtures = ["alphakeks-server-role"])]
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

		let alphakeks = SteamID::from_u64(76561198282622073_u64)?;
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

	#[crate::test(fixtures = ["alphakeks-server-role"])]
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

		let alphakeks = SteamID::from_u64(76561198282622073_u64)?;
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
