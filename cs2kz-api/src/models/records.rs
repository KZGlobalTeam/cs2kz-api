//! This module holds types related to KZ records.

use std::result::Result;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use super::{CourseWithFilter, Player, ServerSummary};

/// A KZ record.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
  "id": 1,
  "player": {
    "steam_id": "STEAM_1:1:161178172",
    "name": "AlphaKeks"
  },
  "course": {
    "id": 1,
    "map_id": 1,
    "map_name": "kz_checkmate",
    "map_stage": 1,
    "mode": "kz_modded",
    "style": "normal",
    "tier": 3
  },
  "teleports": 69,
  "server": {
    "id": 1,
    "name": "Alpha's KZ"
  },
  "bhop_stats": {
    "perfs": 200,
    "bhops_tick0": 100,
    "bhops_tick1": 100,
    "bhops_tick2": 30,
    "bhops_tick3": 10,
    "bhops_tick4": 10,
    "bhops_tick5": 0,
    "bhops_tick6": 0,
    "bhops_tick7": 0,
    "bhops_tick8": 0
  },
  "created_on": "2023-12-10T10:41:01Z",
}))]
pub struct Record {
	/// The record's ID.
	pub id: u64,

	/// The player who set this record.
	pub player: Player,

	/// The course the record was set on.
	pub course: CourseWithFilter,

	/// The amount of teleports used in this run.
	pub teleports: u32,

	/// The server this record was set on.
	pub server: ServerSummary,

	/// BunnyHop statistics for this run.
	pub bhop_stats: BhopStats,

	/// When this record was set.
	pub created_on: DateTime<Utc>,
}

#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BhopStats {
	pub perfs: u16,
	pub bhops_tick0: u16,
	pub bhops_tick1: u16,
	pub bhops_tick2: u16,
	pub bhops_tick3: u16,
	pub bhops_tick4: u16,
	pub bhops_tick5: u16,
	pub bhops_tick6: u16,
	pub bhops_tick7: u16,
	pub bhops_tick8: u16,
}

impl FromRow<'_, MySqlRow> for Record {
	fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;

		let player_id = row.try_get("player_id")?;
		let steam_id =
			SteamID::from_u32(player_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let player_name = row.try_get("player_name")?;
		let player = Player { steam_id, name: player_name };

		let course_id = row.try_get("course_id")?;
		let map_id = row.try_get("map_id")?;
		let map_name = row.try_get("map_name")?;
		let map_stage = row.try_get("map_stage")?;

		let mode = row
			.try_get::<u8, _>("mode_id")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let style = row
			.try_get::<u8, _>("style_id")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let tier = row
			.try_get::<u8, _>("tier")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let course =
			CourseWithFilter { id: course_id, map_id, map_name, map_stage, mode, style, tier };

		let teleports = row.try_get("teleports")?;

		let server_id = row.try_get("server_id")?;
		let server_name = row.try_get("server_name")?;
		let server = ServerSummary { id: server_id, name: server_name };

		let bhop_stats = BhopStats {
			perfs: row.try_get("perfs")?,
			bhops_tick0: row.try_get("bhops_tick0")?,
			bhops_tick1: row.try_get("bhops_tick1")?,
			bhops_tick2: row.try_get("bhops_tick2")?,
			bhops_tick3: row.try_get("bhops_tick3")?,
			bhops_tick4: row.try_get("bhops_tick4")?,
			bhops_tick5: row.try_get("bhops_tick5")?,
			bhops_tick6: row.try_get("bhops_tick6")?,
			bhops_tick7: row.try_get("bhops_tick7")?,
			bhops_tick8: row.try_get("bhops_tick8")?,
		};

		let created_on = row.try_get("created_on")?;

		Ok(Self { id, player, course, teleports, server, bhop_stats, created_on })
	}
}
