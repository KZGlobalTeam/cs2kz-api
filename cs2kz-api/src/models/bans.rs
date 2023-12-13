//! This module holds types related to KZ bans.

use std::result::Result;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::Serialize;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use super::{Player, ServerSummary};

/// A KZ record.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
	"id": 1,
	"player": {
		"steam_id": "STEAM_1:1:161178172",
		"name": "AlphaKeks"
	},
	"reason": "bhop_hack",
	"server": {
		"id": 1,
		"name": "Alpha's KZ"
	},
	"created_on": "2023-12-10T10:41:01Z"
}))]
pub struct Ban {
	/// The ban's ID.
	pub id: u64,

	/// The player who got banned.
	pub player: Player,

	/// The reason for the ban.
	pub reason: String,

	/// The server the ban was issued by.
	pub server: Option<ServerSummary>,

	/// The admin who issued this ban.
	pub banned_by: Option<Player>,

	/// When this ban was issued.
	pub created_on: DateTime<Utc>,

	/// When this ban will expire.
	pub expires_on: Option<DateTime<Utc>>,
}

impl FromRow<'_, MySqlRow> for Ban {
	fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;

		let player_id = row.try_get("player_id")?;
		let steam_id =
			SteamID::from_u32(player_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let player_name = row.try_get("player_name")?;
		let player = Player { steam_id, name: player_name };

		let reason = row.try_get("reason")?;

		let server_id = row.try_get("server_id");
		let server_name = row.try_get("server_name");
		let server = if let (Ok(server_id), Ok(server_name)) = (server_id, server_name) {
			Some(ServerSummary { id: server_id, name: server_name })
		} else {
			None
		};

		let banned_by_steam_id = row.try_get("banned_by_steam_id");
		let banned_by_name = row.try_get("banned_by_name");

		let banned_by = if let (Ok(steam_id), Ok(name)) = (banned_by_steam_id, banned_by_name) {
			let steam_id =
				SteamID::from_u32(steam_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

			Some(Player { steam_id, name })
		} else {
			None
		};

		let created_on = row.try_get("created_on")?;
		let expires_on = row.try_get("expires_on")?;

		Ok(Self { id, player, reason, server, banned_by, created_on, expires_on })
	}
}
