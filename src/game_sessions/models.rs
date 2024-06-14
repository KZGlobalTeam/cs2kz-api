//! Types for modeling game sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::make_id;
use crate::players::Player;
use crate::records::BhopStats;
use crate::servers::ServerInfo;
use crate::time::Seconds;

make_id!(GameSessionID as u64);
make_id!(CourseSessionID as u64);

/// A game session.
///
/// Game sessions start when a player joins a server, and end either when the player disconnects,
/// or when the map changes. They record statistics about playtime, bhops, and potentially other
/// metrics in the future.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct GameSession {
	/// The session's ID.
	pub id: GameSessionID,

	/// The player associated with this session.
	#[sqlx(flatten)]
	pub player: Player,

	/// The server which submitted this session.
	#[sqlx(flatten)]
	pub server: ServerInfo,

	/// Stats about how the player spent their time.
	#[sqlx(flatten)]
	pub time_spent: TimeSpent,

	/// Stats about how many bhops were performed by the player, and how many of them were
	/// perfect bhops.
	#[sqlx(flatten)]
	pub bhop_stats: BhopStats,

	/// When this session was submitted.
	pub created_on: DateTime<Utc>,
}

/// Statistics about how a player spent their time on a KZ server.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct TimeSpent {
	/// How many seconds they were actively playing (had a running timer).
	pub active: Seconds,

	/// How many seconds they were in spectator mode.
	pub spectating: Seconds,

	/// How many seconds they were inactive.
	pub afk: Seconds,
}

impl FromRow<'_, MySqlRow> for TimeSpent {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			active: row.try_get("time_active")?,
			spectating: row.try_get("time_spectating")?,
			afk: row.try_get("time_afk")?,
		})
	}
}
