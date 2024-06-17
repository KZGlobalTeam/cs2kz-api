//! Types for modeling KZ players.

use std::collections::BTreeMap;
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

/// Basic information about a KZ player.
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Player {
	/// The player's name.
	#[sqlx(rename = "player_name")]
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "player_id")]
	pub steam_id: SteamID,
}

/// Detailed information about a KZ player.
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FullPlayer {
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "id")]
	pub steam_id: SteamID,

	/// The player's IP address.
	///
	/// This field is only included if the requesting user has `BANS` permissions.
	#[serde(
		skip_serializing_if = "Option::is_none",
		serialize_with = "FullPlayer::serialize_ip_address",
		deserialize_with = "FullPlayer::deserialize_ip_address"
	)]
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv6Addr>,

	/// Whether this player is currently banned.
	pub is_banned: bool,
}

impl FullPlayer {
	/// Serializes the [`ip_address`] field with respect to IP mapping.
	///
	/// If a player is submitted with an IPv4 address, it will be mapped to an IPv6 address to
	/// be stored in the database. When retrieving this IP address later, it should be mapped
	/// back to IPv4.
	///
	/// [`ip_address`]: FullPlayer::ip_address
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

	/// Deserializes an IP address and maps it to IPv6 if necessary.
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

/// Request payload for creating a new player.
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

/// Request payload for updating an existing player.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's name.
	pub name: String,

	/// The player's IP address.
	#[schema(value_type = String)]
	pub ip_address: IpAddr,

	/// The player's current in-game preferences.
	#[schema(value_type = Object)]
	pub preferences: JsonValue,

	/// Game Session information.
	pub session: Session,
}

/// Game Session information.
///
/// A game session starts when a player joins a server, and ends either when they disconnect or
/// when the map changes.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Session {
	/// Stats about how the player spent their time.
	#[serde(flatten)]
	pub time_spent: TimeSpent,

	/// Stats about how many bhops were performed by the player, and how many of them were
	/// perfect bhops.
	pub bhop_stats: BhopStats,

	/// Per-Course session information.
	#[serde(deserialize_with = "Session::deserialize_course_sessions")]
	#[schema(example = json!({
	  "69": {
	    "classic": {
	      "playtime": 1337,
	      "started_runs": 54,
	      "finished_runs": 8,
	      "bhop_stats": {
	        "bhops": 432,
	        "perfs": 397,
              }
	    }
	  }
	}))]
	pub course_sessions: BTreeMap<CourseID, CourseSessions>,
}

impl Session {
	/// Deserializes course sessions and (partially) validates them.
	///
	/// This function ensures **logical invariants**, such as:
	///    1. no session has more [finished runs] than [started runs]
	///
	/// This function does **not** ensure that IDs actually exist, or belong to appropriate
	/// courses. That validation needs to be done later, as it requires database access.
	///
	/// [finished runs]: CourseSession::finished_runs
	/// [started runs]: CourseSession::started_runs
	fn deserialize_course_sessions<'de, D>(
		deserializer: D,
	) -> Result<BTreeMap<CourseID, CourseSessions>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let sessions = BTreeMap::<CourseID, CourseSessions>::deserialize(deserializer)?;

		if let Some(course_id) = sessions
			.iter()
			.find(|(_, sessions)| !sessions.is_valid())
			.map(|(course_id, _)| course_id)
		{
			return Err(serde::de::Error::custom(format_args!(
				"cannot have more finished runs than started runs for course {course_id}",
			)));
		}

		Ok(sessions)
	}
}

/// Course sessions for all the modes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct CourseSessions {
	/// Course session for vanilla.
	pub vanilla: Option<CourseSession>,

	/// Course session for classic.
	pub classic: Option<CourseSession>,
}

impl CourseSessions {
	/// Checks if these sessions are logically valid.
	fn is_valid(&self) -> bool {
		[&self.vanilla, &self.classic]
			.into_iter()
			.filter_map(Option::as_ref)
			.all(CourseSession::is_valid)
	}
}

impl IntoIterator for CourseSessions {
	type Item = (Mode, CourseSession);
	type IntoIter = CourseSessionsIter;

	fn into_iter(self) -> Self::IntoIter {
		CourseSessionsIter(self)
	}
}

/// Iterator to iterate over the modes and sessions in [`CourseSessions`].
#[derive(Debug)]
pub struct CourseSessionsIter(CourseSessions);

impl Iterator for CourseSessionsIter {
	type Item = (Mode, CourseSession);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(session) = self.0.vanilla.take() {
			return Some((Mode::Vanilla, session));
		}

		if let Some(session) = self.0.classic.take() {
			return Some((Mode::Classic, session));
		}

		None
	}
}

/// Session information tied to a specific course.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct CourseSession {
	/// The amount of seconds the player spent playing this course.
	pub playtime: Seconds,

	/// How many times the player has left the start zone of this course.
	pub started_runs: u16,

	/// How many times the player has entered the end zone of this course.
	pub finished_runs: u16,

	/// Bhop statistics specific to this course.
	pub bhop_stats: BhopStats,
}

impl CourseSession {
	/// Checks if this session is logically valid.
	const fn is_valid(&self) -> bool {
		self.finished_runs <= self.started_runs
	}
}
