//! Types used for describing game sessions and related concepts.
//!
//! Game sessions are recorded while players are playing on global servers, and submitted whenever
//! a player disconnects or when the map changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::records::BhopStats;
use crate::servers::ServerInfo;
use crate::time::Seconds;

/// An in-game session.
///
/// See [module level documentation] for more details.
///
/// [module level documentation]: crate::game_sessions::models
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct GameSession {
	/// The session's ID.
	pub id: u64,

	/// The player associated with the session.
	#[sqlx(flatten)]
	pub player: Player,

	/// The server which submitted this session.
	#[sqlx(flatten)]
	pub server: ServerInfo,

	/// Statistics on how much time the player spent doing what.
	#[sqlx(flatten)]
	pub time_spent: TimeSpent,

	/// Bhop statistics about this session.
	#[sqlx(flatten)]
	pub bhop_stats: BhopStats,

	/// When this session was submitted.
	pub created_on: DateTime<Utc>,
}

/// Breakdown of how time was spent.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimeSpent {
	/// How much time did the player spend actively playing?
	pub active: Seconds,

	/// How much time did the player spend in spectator mode?
	pub spectating: Seconds,

	/// How much time did the player spend doing nothing?
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
