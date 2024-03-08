use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;

use cs2kz::{Mode, SteamID};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::sessions::models::{BhopStats, TimeSpent};

/// Basic information about a KZ player.
///
/// This is included as a field inside many other types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Player {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's latest known name.
	pub name: String,
}

/// Response body for fetching players.
///
/// The [`is_banned`] field is usually not necessary, except in `/players` responses.
///
/// [`is_banned`]: FullPlayer::is_banned
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FullPlayer {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's latest known name.
	pub name: String,

	/// Whether this player is currently banned.
	pub is_banned: bool,
}

/// Request body for registering new KZ players.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewPlayer {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's current name.
	pub name: String,

	/// The player's current IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,
}

/// Request body for updating players.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's new name.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub name: Option<String>,

	/// The player's new IP address.
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv4Addr>,

	/// Data about this session.
	pub session: PlayerUpdateSession,

	/// Data about individual course sessions.
	///
	/// Course ID -> Data
	pub course_sessions: HashMap<u32, PlayerUpdateCourseSession>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdateSession {
	pub time: TimeSpent,
	pub bhop_stats: BhopStats,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdateCourseSession {
	/// The mode that was played.
	pub mode: Mode,

	/// The amount of seconds spent on this course.
	#[serde(with = "crate::serde::duration::as_secs")]
	#[schema(value_type = u16)]
	pub playtime: Duration,

	/// How many times the player left the start zone.
	pub total_runs: u16,

	/// How many times the player entered the end zone.
	pub finished_runs: u16,

	/// Bhop statistics.
	pub bhop_stats: BhopStats,
}
