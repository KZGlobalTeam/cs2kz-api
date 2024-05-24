//! Types used for describing CS2 servers.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use derive_more::Debug;
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::make_id;
use crate::players::Player;

make_id!(ServerID as u16);

/// An approved CS2 server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Server {
	/// The server's ID.
	pub id: ServerID,

	/// The server's name.
	pub name: String,

	/// The server's host.
	#[schema(value_type = String)]
	pub host: url::Host,

	/// The server's port.
	pub port: u16,

	/// The server's owner.
	pub owner: Player,

	/// When this server was approved.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Server {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			name: row.try_get("name")?,
			host: {
				let raw_host = row.try_get::<&str, _>("host")?;
				match raw_host.parse::<IpAddr>() {
					Ok(IpAddr::V4(ip)) => url::Host::Ipv4(ip),
					Ok(IpAddr::V6(ip)) => url::Host::Ipv6(ip),
					Err(_) => url::Host::Domain(raw_host.to_owned()),
				}
			},
			port: row.try_get("port")?,
			owner: Player {
				name: row.try_get("owner_name")?,
				steam_id: row.try_get("owner_id")?,
			},
			created_on: row.try_get("created_on")?,
		})
	}
}

/// Request body for approving new servers.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewServer {
	/// The server's name.
	pub name: String,

	/// The server's host.
	#[schema(value_type = String)]
	pub host: url::Host,

	/// The server's port.
	pub port: u16,

	/// The SteamID of the player who owns this server.
	#[debug("{owned_by}")]
	pub owned_by: SteamID,
}

/// A newly created server.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct CreatedServer {
	/// The server's ID.
	pub server_id: ServerID,

	/// The server's "permanent" refresh key.
	pub refresh_key: Uuid,
}

/// Request body for updating servers.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerUpdate {
	/// A new name.
	pub name: Option<String>,

	/// A new host.
	#[schema(value_type = Option<String>)]
	pub host: Option<url::Host>,

	/// A new port.
	pub port: Option<u16>,

	/// A new owner.
	pub owned_by: Option<SteamID>,
}

/// Request body for generating JWTs.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshKeyRequest {
	/// The server's permanent refresh key.
	pub refresh_key: Uuid,

	/// The CS2KZ version the server is currently running.
	#[schema(value_type = String)]
	pub plugin_version: Version,
}

/// Response body for generating JWTs.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshKeyResponse {
	/// The JWT.
	pub access_key: String,
}

/// Response for generating a new permanent refresh key.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct RefreshKey {
	/// The refresh key.
	pub refresh_key: Uuid,
}

/// Information about a server.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ServerInfo {
	/// The server's ID.
	#[sqlx(rename = "server_id")]
	pub id: ServerID,

	/// The server's name.
	#[sqlx(rename = "server_name")]
	pub name: String,
}
