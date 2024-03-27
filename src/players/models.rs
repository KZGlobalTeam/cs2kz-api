//! Types used for describing players.

use std::collections::{BTreeMap, HashSet};
use std::net::Ipv4Addr;
use std::num::NonZeroU32;
use std::time::Duration;

use cs2kz::{Mode, SteamID};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::game_sessions::models::TimeSpent;
use crate::records::BhopStats;

/// A KZ player.
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Player {
	/// The player's name.
	#[sqlx(rename = "player_name")]
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "player_id")]
	pub steam_id: SteamID,
}

/// A KZ player.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FullPlayer {
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's IP address.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv4Addr>,

	/// Whether the player is currently banned.
	pub is_banned: bool,
}

impl FromRow<'_, MySqlRow> for FullPlayer {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			name: row.try_get("name")?,
			steam_id: row.try_get("id")?,
			ip_address: row
				.try_get::<&str, _>("ip_address")?
				.parse::<Ipv4Addr>()
				.map_err(|err| sqlx::Error::ColumnDecode {
					index: String::from("ip_address"),
					source: Box::new(err),
				})
				.map(Some)?,
			is_banned: row.try_get("is_banned")?,
		})
	}
}

/// Request body for registering new players.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewPlayer {
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,
}

/// Request body for updating players.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's name.
	pub name: String,

	/// The player's IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,

	/// Data about the player's game session.
	pub session: Session,
}

/// Data about the player's game session.
///
/// Whenever a server changes map or when a player disconnects, an update about that player is sent
/// to the API. Between the moment when the player joined, and the moment the server decided to
/// send a player update, a bunch of data is recorded and included in the request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Session {
	/// Statistics about how the player spent their time on the server.
	#[serde(flatten)]
	pub time_spent: TimeSpent,

	/// Bhop statistics about this session.
	pub bhop_stats: BhopStats,

	/// More data grouped by course & mode.
	#[serde(deserialize_with = "Session::deserialize_course_sessions")]
	pub course_sessions: BTreeMap<NonZeroU32, CourseSession>,
}

impl Session {
	/// Deserializes and validates submitted course sessions.
	fn deserialize_course_sessions<'de, D>(
		deserializer: D,
	) -> Result<BTreeMap<NonZeroU32, CourseSession>, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de;

		let course_sessions = BTreeMap::<NonZeroU32, CourseSession>::deserialize(deserializer)?;

		if let Some(course_id) = course_sessions
			.iter()
			.find(|(_, session)| session.finished_runs > session.started_runs)
			.map(|(course_id, _)| course_id)
		{
			return Err(de::Error::custom(format_args!(
				"cannot have more finished runs than started runs for course {course_id}",
			)));
		}

		let mut modes = HashSet::new();

		for mode in course_sessions.values().map(|session| session.mode) {
			if !modes.insert(mode) {
				return Err(de::Error::custom(format_args!(
					"cannot submit duplicate course sessions stats for {mode}",
				)));
			}
		}

		Ok(course_sessions)
	}
}

/// Session data about a specific course.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CourseSession {
	/// The mode the player was playing the course in.
	pub mode: Mode,

	/// How much time the player spent on this course with a running timer.
	#[serde(with = "crate::serde::duration::as_secs")]
	pub playtime: Duration,

	/// How many times the player left the start zone.
	pub started_runs: u16,

	/// How many times the player entered the end zone.
	pub finished_runs: u16,

	/// Bhop statistics about this session.
	pub bhop_stats: BhopStats,
}
