use std::time::Duration;

use chrono::{DateTime, Utc};
use cs2kz::Mode;
use serde::Serialize;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::sessions::models::BhopStats;

/// Response body for course sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct CourseSession {
	/// The session's ID.
	pub id: u32,

	/// The player associated with the session.
	pub player: Player,

	/// The mode that was played.
	pub mode: Mode,

	/// The course's ID.
	pub course_id: u32,

	/// The course's name.
	pub course_name: Option<String>,

	/// The map's ID.
	pub map_id: u32,

	/// The map's name.
	pub map_name: String,

	/// The server's ID.
	pub server_id: u16,

	/// The server's name.
	pub server_name: String,

	/// How many seconds the player spent playing this course.
	#[serde(with = "crate::serde::duration::as_secs")]
	#[schema(value_type = u16)]
	pub playtime: Duration,

	/// How many times the player left the start zone.
	pub total_runs: u16,

	/// How many times the player entered the end zone.
	pub finished_runs: u16,

	/// Bhop statistics.
	pub bhop_stats: BhopStats,

	/// When this session was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for CourseSession {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;

		let player =
			Player { steam_id: row.try_get("steam_id")?, name: row.try_get("player_name")? };

		let mode = row.try_get("mode")?;
		let course_id = row.try_get("course_id")?;
		let course_name = row.try_get("course_name")?;
		let map_id = row.try_get("map_id")?;
		let map_name = row.try_get("map_name")?;
		let server_id = row.try_get("server_id")?;
		let server_name = row.try_get("server_name")?;
		let playtime = row.try_get("playtime").map(Duration::from_secs)?;
		let total_runs = row.try_get("total_runs")?;
		let finished_runs = row.try_get("finished_runs")?;

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

		let created_on = row.try_get("created_on")?;

		Ok(Self {
			id,
			player,
			mode,
			course_id,
			course_name,
			map_id,
			map_name,
			server_id,
			server_name,
			playtime,
			total_runs,
			finished_runs,
			bhop_stats,
			created_on,
		})
	}
}
