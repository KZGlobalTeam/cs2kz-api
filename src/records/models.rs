//! Types for modeling KZ records.

use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Style};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::kz::StyleFlags;
use crate::make_id;
use crate::maps::{CourseID, CourseInfo, MapInfo};
use crate::players::Player;
use crate::servers::ServerInfo;
use crate::time::Seconds;

make_id!(RecordID as u64);

/// A KZ record.
#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
	/// The record's ID.
	pub id: RecordID,

	/// The mode the record was performed in.
	pub mode: Mode,

	/// The styles that were used.
	pub styles: Vec<Style>,

	/// The amount of teleports used.
	pub teleports: u16,

	/// The time in seconds.
	pub time: Seconds,

	/// The player who performed the record.
	pub player: Player,

	/// The map the record was performed on.
	pub map: MapInfo,

	/// The course the record was performed on.
	pub course: CourseInfo,

	/// The server the record was performed on.
	pub server: ServerInfo,

	/// Bhop statistics.
	pub bhop_stats: BhopStats,

	/// When this record was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Record {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			mode: row.try_get("mode")?,
			styles: row
				.try_get("style_flags")
				.map(StyleFlags::new)?
				.into_iter()
				.map(str::parse::<Style>)
				.map(|flags| {
					flags.map_err(|err| sqlx::Error::ColumnDecode {
						index: String::from("style_flags"),
						source: Box::new(err),
					})
				})
				.try_collect()?,
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

/// Bhop statistics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BhopStats {
	/// The amount of bhops.
	pub bhops: u16,

	/// The amount of perfect bhops.
	pub perfs: u16,
}

impl BhopStats {
	/// Deserializes [`BhopStats`] and checks that `perfs <= bhops`.
	pub fn deserialize_checked<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let bhop_stats = Self::deserialize(deserializer)?;

		if bhop_stats.perfs > bhop_stats.bhops {
			return Err(serde::de::Error::custom(
				"bhop stats can't have more perfs than bhops",
			));
		}

		Ok(bhop_stats)
	}
}

/// Request payload for creating a new record.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewRecord {
	/// The SteamID of the player who performed the record.
	pub player_id: SteamID,

	/// The mode the record was performed in.
	pub mode: Mode,

	/// The styles that were used.
	pub styles: Vec<Style>,

	/// ID of the course the record was performed on.
	pub course_id: CourseID,

	/// The amount of teleports used.
	pub teleports: u16,

	/// The time in seconds.
	pub time: Seconds,

	/// Bhop statistics.
	#[serde(deserialize_with = "BhopStats::deserialize_checked")]
	pub bhop_stats: BhopStats,
}

/// Response body for creating a new record.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedRecord {
	/// The record's ID.
	pub record_id: RecordID,
}
