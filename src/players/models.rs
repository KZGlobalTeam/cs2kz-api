//! Types used for describing players.

use std::collections::{BTreeMap, HashSet};
use std::net::{IpAddr, Ipv6Addr};

use cs2kz::{Mode, SteamID};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::game_sessions::TimeSpent;
use crate::maps::CourseID;
use crate::records::BhopStats;
use crate::time::Seconds;

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
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FullPlayer {
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "id")]
	pub steam_id: SteamID,

	/// The player's IP address.
	#[serde(
		skip_serializing_if = "Option::is_none",
		serialize_with = "FullPlayer::serialize_ip_address",
		deserialize_with = "FullPlayer::deserialize_ip_address"
	)]
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv6Addr>,

	/// Whether the player is currently banned.
	pub is_banned: bool,
}

impl FullPlayer {
	/// Serializes the `ip_address` field as an IPv4 address, if it is a mapped IPv4 address.
	///
	/// This is to ensure that, if a player updated submitted an IPv4 address, later retrieval
	/// of that IP address is still an IPv4 address, even though the database only stores
	/// (potentially mapped) IPv6 addresses.
	fn serialize_ip_address<S>(ip: &Option<Ipv6Addr>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		if let Some(ipv4) = ip.and_then(|ip| ip.to_ipv4_mapped()) {
			serializer.serialize_some(&ipv4)
		} else {
			ip.serialize(serializer)
		}
	}

	/// Deserializes a generic IP address, and maps any potential IPv4 addresses to IPv6.
	fn deserialize_ip_address<'de, D>(deserializer: D) -> Result<Option<Ipv6Addr>, D::Error>
	where
		D: Deserializer<'de>,
	{
		Option::<IpAddr>::deserialize(deserializer).map(|ip| {
			ip.map(|ip| match ip {
				IpAddr::V4(ip) => ip.to_ipv6_mapped(),
				IpAddr::V6(ip) => ip,
			})
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
	pub ip_address: IpAddr,
}

/// Request body for updating players.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's name.
	pub name: String,

	/// The player's IP address.
	#[schema(value_type = String)]
	pub ip_address: IpAddr,

	/// Data about the player's game session.
	pub session: Session,

	/// The player's current in-game preference settings.
	pub preferences: JsonValue,
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
	pub course_sessions: BTreeMap<CourseID, CourseSession>,
}

impl Session {
	/// Deserializes and validates submitted course sessions.
	fn deserialize_course_sessions<'de, D>(
		deserializer: D,
	) -> Result<BTreeMap<CourseID, CourseSession>, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de;

		let course_sessions = BTreeMap::<CourseID, CourseSession>::deserialize(deserializer)?;

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct CourseSession {
	/// The mode the player was playing the course in.
	pub mode: Mode,

	/// How much time the player spent on this course with a running timer.
	pub playtime: Seconds,

	/// How many times the player left the start zone.
	pub started_runs: u16,

	/// How many times the player entered the end zone.
	pub finished_runs: u16,

	/// Bhop statistics about this session.
	pub bhop_stats: BhopStats,
}
