use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Style, Tier};
use serde::Serialize;
use utoipa::ToSchema;

/// A KZ record.
#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
	/// The ID of the record.
	pub id: u64,

	/// The map this record was performed on.
	pub map: RecordMap,

	/// The mode this record was performed in.
	pub mode: Mode,

	/// The style this record was performed in.
	pub style: Style,

	/// The player who performed this record.
	pub player: RecordPlayer,

	/// The server this record was performed on.
	pub server: RecordServer,

	/// The amount of teleports used during this run.
	pub teleports: u16,

	/// The time it took to complete this run (in seconds).
	pub time: f64,

	/// Timestamp of when this record was submitted.
	pub created_on: DateTime<Utc>,
}

/// A KZ map.
#[derive(Debug, Serialize, ToSchema)]
pub struct RecordMap {
	/// The ID of the map.
	pub id: u16,

	/// The name of the map.
	pub name: String,

	/// The course this record was performed on.
	pub course: RecordCourse,
}

/// A KZ course.
#[derive(Debug, Serialize, ToSchema)]
pub struct RecordCourse {
	/// The ID of the course.
	pub id: u32,

	/// The stage this course corresponds to.
	pub stage: u8,

	/// The difficulty rating of this course.
	pub tier: Tier,
}

/// A KZ player.
#[derive(Debug, Serialize, ToSchema)]
pub struct RecordPlayer {
	/// The player's Steam name.
	pub name: String,

	/// The player's `SteamID`.
	pub steam_id: SteamID,
}

/// A KZ server.
#[derive(Debug, Serialize, ToSchema)]
pub struct RecordServer {
	/// The server's ID.
	pub id: u16,

	/// The server's name.
	pub name: String,
}
