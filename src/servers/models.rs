use std::net::SocketAddrV4;
use std::num::NonZeroU32;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;

/// Response body for fetching KZ servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Server {
	/// The server's ID.
	pub id: u16,

	/// The server's name.
	pub name: String,

	/// The server's IP address.
	#[schema(value_type = String)]
	pub ip_address: SocketAddrV4,

	/// The player who owns this server.
	pub owned_by: Player,

	/// When this server was approved.
	pub approved_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Server {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		let id = crate::sqlx::non_zero!("id" as u16, row)?;
		let name = row.try_get("name")?;

		let ip_address = row
			.try_get::<&str, _>("ip_address")?
			.parse()
			.map_err(|err| sqlx::Error::ColumnDecode {
				index: String::from("ip_address"),
				source: Box::new(err),
			})?;

		let port = row.try_get("port")?;
		let ip_address = SocketAddrV4::new(ip_address, port);

		let owned_by = Player {
			steam_id: row.try_get("owned_by_steam_id")?,
			name: row.try_get("owned_by_name")?,
		};

		let approved_on = row.try_get("approved_on")?;

		Ok(Self { id, name, ip_address, owned_by, approved_on })
	}
}

/// Request body for newly approved servers.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewServer {
	/// The server's name.
	pub name: String,

	/// The server's IP address.
	#[schema(value_type = String)]
	pub ip_address: SocketAddrV4,

	/// The SteamID of the player who owns this server.
	pub owned_by: SteamID,
}

/// A newly approved KZ server.
///
/// See [`NewServer`].
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedServer {
	/// The server's ID.
	pub server_id: u16,

	/// The server's semi-permanent API Key.
	#[schema(value_type = u32, minimum = 1)]
	pub api_key: NonZeroU32,
}

/// Request body for updates to a server.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ServerUpdate {
	/// A new name for the server.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub name: Option<String>,

	/// A new IP address for the server.
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<SocketAddrV4>,

	/// SteamID of the new owner of the server.
	pub owned_by: Option<SteamID>,
}
