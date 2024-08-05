//! A service for managing KZ servers.

use std::fmt;
use std::time::Duration;

use axum::extract::FromRef;
use sqlx::{MySql, Pool};
use tap::{Pipe, TryConv};

use crate::database::{SqlErrorExt, TransactionExt};
use crate::services::auth::{jwt, Jwt};
use crate::services::plugin::PluginVersionID;
use crate::services::AuthService;
use crate::time::DurationExt;

pub(crate) mod http;
mod queries;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	ApiKey,
	DeleteKeyRequest,
	DeleteKeyResponse,
	FetchServerRequest,
	FetchServerResponse,
	FetchServersRequest,
	FetchServersResponse,
	GenerateAccessTokenRequest,
	GenerateAccessTokenResponse,
	Host,
	RegisterServerRequest,
	RegisterServerResponse,
	ResetKeyRequest,
	ResetKeyResponse,
	ServerID,
	ServerInfo,
	ServerOwner,
	UpdateServerRequest,
	UpdateServerResponse,
};

/// A service for managing KZ servers.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct ServerService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl fmt::Debug for ServerService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("ServerService").finish_non_exhaustive()
	}
}

impl ServerService
{
	/// Create a new [`ServerService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetch information about a server.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_server(&self, req: FetchServerRequest)
	-> Result<Option<FetchServerResponse>>
	{
		let res = sqlx::query_as::<_, FetchServerResponse>(&format!(
			r"
			{}
			WHERE
			  s.id = COALESCE(?, s.id)
			  AND s.name LIKE COALESCE(?, s.name)
			LIMIT
			  1
			",
			queries::SELECT,
		))
		.bind(req.identifier.as_id())
		.bind(req.identifier.as_name().map(|name| format!("%{name}%")))
		.fetch_optional(&self.database)
		.await?;

		Ok(res)
	}

	/// Fetch information about servers.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_servers(&self, req: FetchServersRequest) -> Result<FetchServersResponse>
	{
		let mut txn = self.database.begin().await?;
		let owner_id = match req.owned_by {
			None => None,
			Some(player) => Some(player.resolve_id(txn.as_mut()).await?),
		};

		let servers = sqlx::query_as::<_, FetchServerResponse>(&format!(
			r"
			{}
			WHERE
			  s.name LIKE COALESCE(?, s.name)
			  AND s.host = COALESCE(?, s.host)
			  AND s.owner_id = COALESCE(?, s.owner_id)
			  AND s.created_on > COALESCE(?, '1970-01-01 00:00:01')
			  AND s.created_on < COALESCE(?, '2038-01-19 03:14:07')
			LIMIT
			  ? OFFSET ?
			",
			queries::SELECT,
		))
		.bind(req.name)
		.bind(req.host)
		.bind(owner_id)
		.bind(req.created_after)
		.bind(req.created_before)
		.bind(*req.limit)
		.bind(*req.offset)
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchServersResponse { servers, total })
	}

	/// Register a new server.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn register_server(
		&self,
		req: RegisterServerRequest,
	) -> Result<RegisterServerResponse>
	{
		let mut txn = self.database.begin().await?;
		let api_key = ApiKey::new();

		let server_id = sqlx::query! {
			r"
			INSERT INTO
			  Servers (name, host, port, owner_id, `key`)
			VALUES
			  (?, ?, ?, ?, ?)
			",
			req.name,
			req.host,
			req.port,
			req.owner_id,
			api_key,
		}
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("owner_id") {
				Error::ServerOwnerDoesNotExist { steam_id: req.owner_id }
			} else {
				Error::Database(error)
			}
		})?
		.last_insert_id()
		.try_conv::<ServerID>()
		.expect("in-range ID");

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			%api_key,
			%server_id,
			owner_id = %req.owner_id,
			"registered new server",
		};

		Ok(RegisterServerResponse { server_id, api_key })
	}

	/// Update a server.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn update_server(&self, req: UpdateServerRequest) -> Result<UpdateServerResponse>
	{
		if req.is_empty() {
			return Ok(UpdateServerResponse { _priv: () });
		}

		let mut txn = self.database.begin().await?;

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Servers
			SET
			  name = COALESCE(?, name),
			  host = COALESCE(?, host),
			  port = COALESCE(?, port),
			  owner_id = COALESCE(?, owner_id)
			WHERE
			  id = ?
			",
			req.new_name,
			req.new_host,
			req.new_port,
			req.new_owner,
			req.server_id
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::ServerDoesNotExist),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			server_id = %req.server_id,
			"updated server",
		};

		Ok(UpdateServerResponse { _priv: () })
	}

	/// Resets a server's API key.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn reset_key(&self, req: ResetKeyRequest) -> Result<ResetKeyResponse>
	{
		let mut txn = self.database.begin().await?;

		let new_key = ApiKey::new();

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Servers
			SET
			  `key` = ?
			WHERE
			  id = ?
			",
			new_key,
			req.server_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::ServerDoesNotExist),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			server_id = %req.server_id,
			%new_key,
			"reset API key for server",
		};

		Ok(ResetKeyResponse { key: new_key })
	}

	/// Delete a server's API key.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn delete_key(&self, req: DeleteKeyRequest) -> Result<DeleteKeyResponse>
	{
		let mut txn = self.database.begin().await?;

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Servers
			SET
			  `key` = NULL
			WHERE
			  id = ?
			",
			req.server_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::ServerDoesNotExist),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			server_id = %req.server_id,
			"deleted API key of server",
		};

		Ok(DeleteKeyResponse { _priv: () })
	}

	/// Generate a temporary access token for a CS2 server.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn generate_access_token(
		&self,
		req: GenerateAccessTokenRequest,
	) -> Result<GenerateAccessTokenResponse>
	{
		let mut txn = self.database.begin().await?;

		let server_info = sqlx::query! {
			r"
			SELECT
			  s.id `server_id: ServerID`,
			  v.id `plugin_version_id: PluginVersionID`
			FROM
			  Servers s
			  JOIN PluginVersions v ON v.semver = ?
			  AND s.key = ?
			",
			req.plugin_version,
			req.key,
		}
		.fetch_optional(txn.as_mut())
		.await?
		.map(|row| jwt::ServerInfo::new(row.server_id, row.plugin_version_id))
		.ok_or(Error::InvalidKeyOrPluginVersion)?;

		sqlx::query! {
			r"
			UPDATE
			  Servers
			SET
			  last_seen_on = NOW()
			WHERE
			  id = ?
			",
			server_info.id(),
		}
		.execute(txn.as_mut())
		.await?;

		let token = Jwt::new(&server_info, Duration::MINUTE * 15)
			.pipe(|jwt| self.auth_svc.encode_jwt(jwt))?;

		txn.commit().await?;

		tracing::trace!(server_id = %server_info.id(), %token, "generated jwt");

		Ok(GenerateAccessTokenResponse { token })
	}
}
