//! Types used for describing game sessions and related concepts.
//!
//! Game sessions are recorded while players are playing on global servers, and submitted whenever
//! a player disconnects or when the map changes.

use std::num::NonZeroU64;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::records::BhopStats;
use crate::servers::ServerInfo;
use crate::sqlx::Seconds;

/// An in-game session.
///
/// See [module level documentation] for more details.
///
/// [module level documentation]: crate::game_sessions::models
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct GameSession {
	/// The session's ID.
	#[sqlx(try_from = "u64")]
	#[schema(value_type = u64)]
	pub id: NonZeroU64,

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
	#[serde(with = "crate::serde::duration::as_secs")]
	pub active: Duration,

	/// How much time did the player spend in spectator mode?
	#[serde(with = "crate::serde::duration::as_secs")]
	pub spectating: Duration,

	/// How much time did the player spend doing nothing?
	#[serde(with = "crate::serde::duration::as_secs")]
	pub afk: Duration,
}

impl FromRow<'_, MySqlRow> for TimeSpent {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		let decode = |name: &str| row.try_get::<Seconds, _>(name).map(Into::into);

		Ok(Self {
			active: decode("time_active")?,
			spectating: decode("time_spectating")?,
			afk: decode("time_afk")?,
		})
	}
}
