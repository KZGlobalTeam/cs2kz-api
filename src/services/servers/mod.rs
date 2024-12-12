//! A service for managing KZ servers.

use std::fmt;
use std::time::Duration;

use axum::extract::FromRef;
use itertools::Itertools;
use sqlx::{MySql, Pool, Row};
use tap::{Pipe, Tap, TryConv};

use crate::database::SqlErrorExt;
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
		let raw_servers = sqlx::query_as::<_, FetchServerResponse>(&format!(
			r"
			{}
			WHERE
			  s.id = COALESCE(?, s.id)
			  AND s.name LIKE COALESCE(?, s.name)
			",
			queries::SELECT,
		))
		.bind(req.identifier.as_id())
		.bind(req.identifier.as_name().map(|name| format!("%{name}%")))
		.fetch_all(&self.database)
		.await?;

		let Some(server_id) = raw_servers.first().map(|s| s.id) else {
			return Ok(None);
		};

		let server = raw_servers
			.into_iter()
			.filter(|s| s.id == server_id)
			.reduce(reduce_chunk)
			.expect("we got the id we're filtering by from the original list");

		Ok(Some(server))
	}

	/// Fetch information about servers.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_servers(&self, req: FetchServersRequest) -> Result<FetchServersResponse>
	{
		let owner_id = match req.owned_by {
			None => None,
			Some(player) => Some(player.resolve_id(&self.database).await?),
		};

		let server_count = sqlx::query_scalar!("SELECT COUNT(id) FROM Servers")
			.fetch_one(&self.database)
			.await?
			.try_conv::<u64>()
			.expect("positive count");

		if *req.offset >= server_count {
			return Ok(FetchServersResponse { servers: Vec::new(), total: server_count });
		}

		let server_chunks = sqlx::query_as::<_, FetchServerResponse>(&format!(
			r"
			{}
			WHERE
			  s.name LIKE COALESCE(?, s.name)
			  AND s.host LIKE COALESCE(?, s.host)
			  AND s.owner_id = COALESCE(?, s.owner_id)
			  AND s.created_on > COALESCE(?, '1970-01-01 00:00:01')
			  AND s.created_on < COALESCE(?, '2038-01-19 03:14:07')
			",
			queries::SELECT,
		))
		.bind(req.name.map(|name| format!("%{name}%")))
		.bind(req.host)
		.bind(owner_id)
		.bind(req.created_after)
		.bind(req.created_before)
		.fetch_all(&self.database)
		.await?
		.into_iter()
		.chunk_by(|s| s.id);

		// Take into account how many maps we're gonna skip over
		let mut total = *req.offset;

		let servers = server_chunks
			.into_iter()
			.map(|(_, chunk)| chunk.reduce(reduce_chunk).expect("chunk can't be empty"))
			.skip(*req.offset as usize)
			.take(*req.limit as usize)
			.collect_vec();

		total += servers.len() as u64;
		total += server_chunks.into_iter().count() as u64;

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
			RETURNING id
			",
			req.name,
			req.host,
			req.port,
			req.owner_id,
			api_key,
		}
		.fetch_one(txn.as_mut())
		.await
		.and_then(|row| row.try_get(0))
		.map_err(|error| {
			if error.is_fk_violation("owner_id") {
				Error::ServerOwnerDoesNotExist { steam_id: req.owner_id }
			} else {
				Error::Database(error)
			}
		})?;

		txn.commit().await?;

		tracing::info!(%api_key, %server_id, owner_id = %req.owner_id, "registered new server");

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

		tracing::info!(server_id = %req.server_id, "updated server");

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

		tracing::info!(server_id = %req.server_id, %new_key, "reset API key for server");

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

		tracing::info!(server_id = %req.server_id, "deleted API key of server");

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

/// Reduce function for merging multiple database results for the same server
/// with different tags.
///
/// When we fetch servers from the DB, we get "duplicates" for servers with
/// different tags, since SQL doesn't support arrays. All the other information
/// is the same, except for the tags. We group results by their ID and then
/// reduce each chunk down into a single server that contains all the tags using
/// this function.
fn reduce_chunk(acc: FetchServerResponse, curr: FetchServerResponse) -> FetchServerResponse
{
	assert_eq!(acc.id, curr.id, "merging two unrelated servers");

	acc.tap_mut(|acc| acc.tags.0.extend(curr.tags.0))
}

#[cfg(test)]
mod tests
{
	use cs2kz::SteamID;
	use sqlx::{MySql, Pool};

	use super::*;
	use crate::services::plugin::PluginVersion;
	use crate::testing::{self, ALPHAKEKS_ID};

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_server_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = FetchServerRequest { identifier: "alpha".parse()? };
		let res = svc.fetch_server(req).await?;

		testing::assert!(res.as_ref().is_some_and(|s| s.name == "Alpha's KZ"));

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_server_not_found(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = FetchServerRequest { identifier: "foobar".parse()? };
		let res = svc.fetch_server(req).await?;

		testing::assert!(res.is_none());

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/servers.sql")
	)]
	async fn fetch_servers_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req =
			FetchServersRequest { name: Some(String::from("balls.kz EU")), ..Default::default() };
		let res = svc.fetch_servers(req).await?;

		testing::assert_eq!(res.servers.len(), 2);
		testing::assert_eq!(res.total, 2);

		let req = FetchServersRequest { host: Some(".balls.com".parse()?), ..Default::default() };
		let res = svc.fetch_servers(req).await?;

		testing::assert_eq!(res.servers.len(), 3);
		testing::assert_eq!(res.total, 3);

		let req =
			FetchServersRequest { owned_by: Some("AlphaKeks".parse()?), ..Default::default() };
		let res = svc.fetch_servers(req).await?;

		testing::assert_eq!(res.servers.len(), 1);
		testing::assert_eq!(res.total, 1);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn register_server_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = RegisterServerRequest {
			name: String::from("my cool new server!"),
			host: "123.456.789.420".parse()?,
			port: 1337,
			owner_id: ALPHAKEKS_ID,
		};

		let res = svc.register_server(req).await?;
		let server = svc
			.fetch_server(FetchServerRequest { identifier: res.server_id.into() })
			.await?
			.expect("server should be available to fetch after registering");

		testing::assert_eq!(server.name, "my cool new server!");
		testing::assert_eq!(server.owner.steam_id, ALPHAKEKS_ID);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn register_server_rejects_unknown_owner(database: Pool<MySql>)
		-> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let ibrahizy_id = SteamID::new(76561198264939817).unwrap();
		let req = RegisterServerRequest {
			name: String::from("my cool new server!"),
			host: "123.456.789.420".parse()?,
			port: 1337,
			owner_id: ibrahizy_id,
		};

		let res = svc.register_server(req).await.unwrap_err();

		testing::assert_matches!(
			res,
			Error::ServerOwnerDoesNotExist { steam_id }
				if steam_id == ibrahizy_id
		);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_server_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = UpdateServerRequest {
			server_id: 1.into(),
			new_name: Some(String::from("new name")),
			new_host: None,
			new_port: None,
			new_owner: None,
		};

		let _res = svc.update_server(req).await?;

		let name = sqlx::query_scalar!("SELECT name FROM Servers WHERE id = 1")
			.fetch_one(&svc.database)
			.await?;

		testing::assert_eq!(name, "new name");

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_server_rejects_unknown_server(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = UpdateServerRequest {
			server_id: 69.into(),
			new_name: Some(String::from("new name")),
			new_host: None,
			new_port: None,
			new_owner: None,
		};

		let res = svc.update_server(req).await.unwrap_err();

		testing::assert_matches!(res, Error::ServerDoesNotExist);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn reset_key_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);

		let old_key = sqlx::query_scalar!("SELECT `key` `key!: ApiKey` FROM Servers WHERE id = 1")
			.fetch_one(&svc.database)
			.await?;

		let req = ResetKeyRequest { server_id: 1.into() };
		let res = svc.reset_key(req).await?;

		testing::assert_ne!(res.key, old_key);

		let new_key = sqlx::query_scalar!("SELECT `key` `key!: ApiKey` FROM Servers WHERE id = 1")
			.fetch_one(&svc.database)
			.await?;

		testing::assert_eq!(new_key, res.key);
		testing::assert_ne!(new_key, old_key);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn reset_key_rejects_unknown_server(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = ResetKeyRequest { server_id: 69.into() };
		let res = svc.reset_key(req).await.unwrap_err();

		testing::assert_matches!(res, Error::ServerDoesNotExist);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn delete_key_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let old_key = sqlx::query_scalar!("SELECT `key` `key: ApiKey` FROM Servers WHERE id = 1")
			.fetch_one(&svc.database)
			.await?;

		testing::assert!(old_key.is_some());

		let req = DeleteKeyRequest { server_id: 1.into() };
		let _res = svc.delete_key(req).await?;

		let new_key = sqlx::query_scalar!("SELECT `key` `key: ApiKey` FROM Servers WHERE id = 1")
			.fetch_one(&svc.database)
			.await?;

		testing::assert!(new_key.is_none());

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn delete_key_rejects_unknown_server(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req = DeleteKeyRequest { server_id: 69.into() };
		let res = svc.delete_key(req).await.unwrap_err();

		testing::assert_matches!(res, Error::ServerDoesNotExist);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn generate_access_token_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);

		let (key, plugin_version) = sqlx::query! {
			r"
			SELECT
			  s.`key` `key!: ApiKey`,
			  v.semver `plugin_version: PluginVersion`
			FROM
			  Servers s
			  JOIN PluginVersions v
			WHERE
			  s.id = 1
			ORDER BY
			  v.created_on DESC
			",
		}
		.fetch_one(&svc.database)
		.await
		.map(|row| (row.key, row.plugin_version))?;

		let req = GenerateAccessTokenRequest { key, plugin_version };
		let res = svc.generate_access_token(req).await;

		testing::assert!(res.is_ok());

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn generate_access_token_rejects_invalid_key(
		database: Pool<MySql>,
	) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);
		let req =
			GenerateAccessTokenRequest { key: ApiKey::new(), plugin_version: "0.0.0".parse()? };

		let res = svc.generate_access_token(req).await.unwrap_err();

		testing::assert_matches!(res, Error::InvalidKeyOrPluginVersion);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn generate_access_token_rejects_invalid_version(
		database: Pool<MySql>,
	) -> color_eyre::Result<()>
	{
		let svc = testing::server_svc(database);

		let key = sqlx::query_scalar!("SELECT `key` `key!: ApiKey` FROM Servers WHERE id = 1")
			.fetch_one(&svc.database)
			.await?;

		let req = GenerateAccessTokenRequest { key, plugin_version: "0.0.0".parse()? };
		let res = svc.generate_access_token(req).await.unwrap_err();

		testing::assert_matches!(res, Error::InvalidKeyOrPluginVersion);

		Ok(())
	}
}
