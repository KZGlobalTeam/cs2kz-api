use std::net::SocketAddrV4;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Server {
	pub id: u16,
	pub name: String,

	#[schema(value_type = String)]
	pub ip_address: SocketAddrV4,

	pub owned_by: Player,
	pub approved_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Server {
	fn from_row(row: &'_ MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;
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

/// A newly registered CS2 server.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedServer {
	/// The server's ID.
	pub server_id: u16,

	/// The server's semi-permanent API Key.
	pub api_key: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ServerUpdate {
	/// A new name for the server.
	pub name: Option<String>,

	/// A new IP address for the server.
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<SocketAddrV4>,

	/// SteamID of the new owner of the server.
	pub owned_by: Option<SteamID>,
}
