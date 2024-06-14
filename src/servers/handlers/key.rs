//! HTTP handlers for the `/servers/key` routes.

use std::time::Duration;

use axum::extract::Path;
use axum::Json;
use uuid::Uuid;

use crate::authentication::{self, Jwt};
use crate::authorization::Permissions;
use crate::openapi::responses::{self, Created, NoContent};
use crate::plugin::PluginVersionID;
use crate::servers::{AccessKeyRequest, AccessKeyResponse, RefreshKey, ServerID};
use crate::{authorization, Error, Result, State};

/// Generate a temporary access token using a CS2 server's API key.
///
/// This endpoint is for CS2 servers. They will generate a new access token every ~30min.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/servers/key",
  tag = "Servers",
  responses(
    responses::Created<Jwt<authentication::Server>>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn generate_temp(
	state: State,
	Json(AccessKeyRequest {
		refresh_key,
		plugin_version,
	}): Json<AccessKeyRequest>,
) -> Result<Created<Json<AccessKeyResponse>>> {
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
	.ok_or_else(|| Error::invalid("token"))?;

	let jwt = Jwt::new(&server, Duration::from_secs(60 * 15));
	let access_key = state.encode_jwt(jwt)?;

	transaction.commit().await?;

	tracing::debug! {
		server_id = %server.id(),
		%access_key,
		"generated access key for server",
	};

	Ok(Created(Json(AccessKeyResponse { access_key })))
}

/// Generate a new API key for a server, invalidating the old one.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
  ),
)]
pub async fn put_perma(
	state: State,
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

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("server ID")),
		n => assert_eq!(n, 1, "updated more than 1 server"),
	}

	transaction.commit().await?;

	tracing::info! {
		target: "cs2kz_api::audit_log",
		%server_id,
		%refresh_key,
		"generated new API key for server",
	};

	Ok(Created(Json(RefreshKey { refresh_key })))
}

/// Delete a server's API key, preventing them from generating new JWTs.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
  ),
)]
pub async fn delete_perma(
	state: State,
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

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("server ID")),
		n => assert_eq!(n, 1, "updated more than 1 server"),
	}

	transaction.commit().await?;

	tracing::info!(target: "cs2kz_api::audit_log", %server_id, "deleted API key for server");

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
	use crate::servers::{AccessKeyRequest, AccessKeyResponse, RefreshKey, ServerID};

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

		let refresh_key = AccessKeyRequest {
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

		let AccessKeyResponse { access_key } = response.json().await?;
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
