//! Types used for describing records ("runs") and related concepts.

use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Style};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::maps::{CourseInfo, MapInfo};
use crate::players::Player;
use crate::servers::ServerInfo;
use crate::time::Seconds;

/// A record (or "run").
#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
	/// The record's ID:
	pub id: u64,

	/// The mode this run was performed in.
	pub mode: Mode,

	/// The style this run was performed in.
	pub style: Style,

	/// The amount of teleports used during this run.
	pub teleports: u16,

	/// The time it took to complete this run.
	pub time: Seconds,

	/// The player who performed this run.
	pub player: Player,

	/// The map this run was performed on.
	pub map: MapInfo,

	/// The course this run was performed on.
	pub course: CourseInfo,

	/// The server this run was performed on.
	pub server: ServerInfo,

	/// Bhop statistics about this run.
	pub bhop_stats: BhopStats,

	/// When this run was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Record {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			mode: row.try_get("mode")?,
			style: row.try_get("style")?,
			teleports: row.try_get("teleports")?,
			time: row.try_get("time")?,
			player: Player::from_row(row)?,
			map: MapInfo::from_row(row)?,
			course: CourseInfo::from_row(row)?,
			server: ServerInfo::from_row(row)?,
			bhop_stats: BhopStats::from_row(row)?,
			created_on: row.try_get("created_on")?,
		})
	}
}

/// Bhop statistics over a certain time period.
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BhopStats {
	/// The amount of perfect Bhops.
	pub perfs: u16,

	/// The amount of scrolls at the exact same tick the player hit the ground.
	#[sqlx(rename = "bhops_tick0")]
	pub tick0: u16,

	/// The amount of scrolls 1 tick after the player hit the ground.
	#[sqlx(rename = "bhops_tick1")]
	pub tick1: u16,

	/// The amount of scrolls 2 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick2")]
	pub tick2: u16,

	/// The amount of scrolls 3 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick3")]
	pub tick3: u16,

	/// The amount of scrolls 4 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick4")]
	pub tick4: u16,

	/// The amount of scrolls 5 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick5")]
	pub tick5: u16,

	/// The amount of scrolls 6 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick6")]
	pub tick6: u16,

	/// The amount of scrolls 7 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick7")]
	pub tick7: u16,

	/// The amount of scrolls 8 ticks after the player hit the ground.
	#[sqlx(rename = "bhops_tick8")]
	pub tick8: u16,
}

/// Request body for submitting new records.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewRecord {
	/// The SteamID of the player who performed this run.
	pub player_id: SteamID,

	/// The mode this run was performed in.
	pub mode: Mode,

	/// The style this run was performed in.
	pub style: Style,

	/// The ID of the course this run was performed on.
	pub course_id: u32,

	/// The amount of teleports used during this run.
	pub teleports: u16,

	/// The time it took to complete this run.
	pub time: Seconds,

	/// Bhop statistics about this run.
	pub bhop_stats: BhopStats,
}

/// A newly created record.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedRecord {
	/// The record's ID.
	pub record_id: u64,
}
