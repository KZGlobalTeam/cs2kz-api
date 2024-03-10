use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Style};
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;

#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
	/// The record's ID.
	pub id: u64,

	/// The player who performed this run.
	pub player: Player,

	/// The map this run was performed on.
	pub map: MapInfo,

	/// The server this run was performed on.
	pub server: ServerInfo,

	/// The mode this run was performed in.
	pub mode: Mode,

	/// The style this run was performed in.
	pub style: Style,

	/// The amount of teleports used during this run.
	pub teleports: u16,

	/// The time it took to complete this run.
	pub time: f64,

	/// Bhop statistics.
	pub bhop_stats: BhopStats,

	/// The CS2KZ plugin version this run was performed on.
	#[schema(value_type = String)]
	pub plugin_version: Version,

	/// When this run was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Record {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;
		let player = Player {
			steam_id: row.try_get("player_id")?,
			name: row.try_get("player_name")?,
		};

		let map = MapInfo { id: row.try_get("map_id")?, name: row.try_get("map_name")? };

		let server =
			ServerInfo { id: row.try_get("server_id")?, name: row.try_get("server_name")? };

		let mode = row.try_get("mode")?;
		let style = row.try_get("style")?;
		let teleports = row.try_get("teleports")?;
		let time = row.try_get("time")?;

		let bhop_stats = BhopStats {
			perfs: row.try_get("perfs")?,
			tick0: row.try_get("bhops_tick0")?,
			tick1: row.try_get("bhops_tick1")?,
			tick2: row.try_get("bhops_tick2")?,
			tick3: row.try_get("bhops_tick3")?,
			tick4: row.try_get("bhops_tick4")?,
			tick5: row.try_get("bhops_tick5")?,
			tick6: row.try_get("bhops_tick6")?,
			tick7: row.try_get("bhops_tick7")?,
			tick8: row.try_get("bhops_tick8")?,
		};

		let plugin_version = row
			.try_get::<&str, _>("plugin_version")?
			.parse::<Version>()
			.map_err(|err| sqlx::Error::ColumnDecode {
				index: String::from("plugin_version"),
				source: Box::new(err),
			})?;

		let created_on = row.try_get("created_on")?;

		Ok(Self {
			id,
			player,
			map,
			server,
			mode,
			style,
			teleports,
			time,
			bhop_stats,
			plugin_version,
			created_on,
		})
	}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MapInfo {
	pub id: u16,
	pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServerInfo {
	pub id: u16,
	pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BhopStats {
	pub perfs: u16,
	pub tick0: u16,
	pub tick1: u16,
	pub tick2: u16,
	pub tick3: u16,
	pub tick4: u16,
	pub tick5: u16,
	pub tick6: u16,
	pub tick7: u16,
	pub tick8: u16,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewRecord {
	pub course_id: u32,
	pub steam_id: SteamID,
	pub mode: Mode,
	pub style: Style,
	pub teleports: u16,
	pub time: f64,
	pub bhop_stats: BhopStats,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedRecord {
	pub record_id: u64,
}
