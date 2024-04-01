//! Types used for describing CS2 servers.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::num::NonZeroU16;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use derive_more::Debug;
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::players::Player;
use crate::sqlx::query;

/// An approved CS2 server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Server {
	/// The server's ID.
	#[schema(value_type = u16)]
	pub id: NonZeroU16,

	/// The server's name.
	pub name: String,

	/// The server's IP address and port.
	#[schema(value_type = String)]
	pub ip_address: SocketAddrV4,

	/// The server's owner.
	pub owner: Player,

	/// When this server was approved.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Server {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: query::non_zero!("id" as NonZeroU16, row)?,
			name: row.try_get("name")?,
			ip_address: {
				let ip = row
					.try_get::<&str, _>("ip_address")?
					.parse::<Ipv4Addr>()
					.map_err(|err| sqlx::Error::ColumnDecode {
						index: String::from("ip_address"),
						source: Box::new(err),
					})?;

				let port = row.try_get("port")?;

				SocketAddrV4::new(ip, port)
			},
			owner: Player { name: row.try_get("owner_name")?, steam_id: row.try_get("owner_id")? },
			created_on: row.try_get("created_on")?,
		})
	}
}

/// Request body for approving new servers.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewServer {
	/// The server's name.
	pub name: String,

	/// The server's IP address and port.
	#[schema(value_type = String)]
	pub ip_address: SocketAddrV4,

	/// The SteamID of the player who owns this server.
	#[debug("{owned_by}")]
	pub owned_by: SteamID,
}

/// A newly created server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedServer {
	/// The server's ID.
	#[schema(value_type = u16)]
	pub server_id: NonZeroU16,

	/// The server's "permanent" refresh key.
	pub refresh_key: Uuid,
}

/// Request body for updating servers.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerUpdate {
	/// A new name.
	pub name: Option<String>,

	/// A new IP address and port.
	pub ip_address: Option<SocketAddrV4>,

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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshKey {
	/// The refresh key.
	pub refresh_key: Uuid,
}

/// Information about a server.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ServerInfo {
	/// The server's ID.
	#[sqlx(rename = "server_id", try_from = "u16")]
	#[schema(value_type = u16)]
	pub id: NonZeroU16,

	/// The server's name.
	#[sqlx(rename = "server_name")]
	pub name: String,
}
