//! This module holds types related to KZ servers.

use std::net::SocketAddrV4;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::Serialize;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use super::players::Player;

/// A server ID and name.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
  "id": 1,
  "name": "Alpha's KZ",
}))]
pub struct ServerSummary {
	/// The server's ID.
	pub id: u16,

	/// The server's name.
	pub name: String,
}

/// Information about a server.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
  "id": 1,
  "name": "Alpha's KZ",
  "ip_address": "255.255.255.255:1337",
  "owned_by": {
    "steam_id": "STEAM_1:1:161178172",
    "name": "AlphaKeks"
  },
  "approved_on": "2023-12-10T10:41:01Z"
}))]
pub struct ServerResponse {
	/// The server's ID.
	id: u16,

	/// The server's name.
	name: String,

	/// The server's IP address and port.
	#[schema(value_type = String)]
	ip_address: SocketAddrV4,

	/// The player who owns this server.
	owned_by: Player,

	/// When this server was approved.
	approved_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for ServerResponse {
	fn from_row(row: &'_ MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;
		let name = row.try_get("name")?;

		let ip_address = row
			.try_get::<&str, _>("ip_address")?
			.parse()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let port = row.try_get("port")?;
		let approved_on = row.try_get("approved_on")?;

		let owner_steam_id = row.try_get("owner_steam_id")?;
		let owner_steam_id =
			SteamID::from_u32(owner_steam_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let owner_name = row.try_get("owner_name")?;

		Ok(Self {
			id,
			name,
			ip_address: SocketAddrV4::new(ip_address, port),
			owned_by: Player { steam_id: owner_steam_id, name: owner_name },
			approved_on,
		})
	}
}
