//! Types for modeling jumpstats.

use chrono::{DateTime, Utc};
use cs2kz::{JumpType, Mode, SteamID};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::make_id;
use crate::players::Player;
use crate::servers::ServerInfo;
use crate::time::Seconds;

make_id!(JumpstatID as u64);

/// A KZ jumpstat.
#[derive(Debug, Serialize, ToSchema)]
pub struct Jumpstat {
	/// The jumpstat's ID.
	pub id: JumpstatID,

	/// The jump type.
	#[serde(rename = "type")]
	pub jump_type: JumpType,

	/// The mode the jump was performed in.
	pub mode: Mode,

	/// The player who performed the jump.
	pub player: Player,

	/// The server the jump was performed on.
	pub server: ServerInfo,

	/// How many strafes the player performed during the jump.
	pub strafes: u8,

	/// The distance cleared by the jump.
	pub distance: f32,

	/// The % of airtime spent gaining speed.
	pub sync: f32,

	/// The speed at jumpoff.
	pub pre: f32,

	/// The maximum speed during the jump.
	pub max: f32,

	/// The amount of time spent pressing both strafe keys.
	pub overlap: Seconds,

	/// The amount of time spent pressing keys but not gaining speed.
	pub bad_angles: Seconds,

	/// The amount of time spent doing nothing.
	pub dead_air: Seconds,

	/// The maximum height reached during the jump.
	pub height: f32,

	/// How close to a perfect airpath this jump was.
	///
	/// The closer to 1.0 the better.
	pub airpath: f32,

	/// How far the landing position deviates from the jumpoff position.
	pub deviation: f32,

	/// The average strafe width.
	pub average_width: f32,

	/// The amount of time spent mid-air.
	pub airtime: Seconds,

	/// When this jumpstat was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Jumpstat {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			jump_type: row.try_get("type")?,
			mode: row.try_get("mode")?,
			player: Player::from_row(row)?,
			server: ServerInfo::from_row(row)?,
			strafes: row.try_get("strafes")?,
			distance: row.try_get("distance")?,
			sync: row.try_get("sync")?,
			pre: row.try_get("pre")?,
			max: row.try_get("max")?,
			overlap: row.try_get("overlap")?,
			bad_angles: row.try_get("bad_angles")?,
			dead_air: row.try_get("dead_air")?,
			height: row.try_get("height")?,
			airpath: row.try_get("airpath")?,
			deviation: row.try_get("deviation")?,
			average_width: row.try_get("average_width")?,
			airtime: row.try_get("airtime")?,
			created_on: row.try_get("created_on")?,
		})
	}
}

/// Request payload for creating a new jumpstat.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct NewJumpstat {
	/// The jump type.
	#[serde(rename = "type")]
	pub jump_type: JumpType,

	/// The mode the jump was performed in.
	pub mode: Mode,

	/// The SteamID of the player who performed the jump.
	pub player_id: SteamID,

	/// How many strafes the player performed during the jump.
	pub strafes: u8,

	/// The distance cleared by the jump.
	pub distance: f32,

	/// The % of airtime spent gaining speed.
	pub sync: f32,

	/// The speed at jumpoff.
	pub pre: f32,

	/// The maximum speed during the jump.
	pub max: f32,

	/// The amount of time spent pressing both strafe keys.
	pub overlap: Seconds,

	/// The amount of time spent pressing keys but not gaining speed.
	pub bad_angles: Seconds,

	/// The amount of time spent doing nothing.
	pub dead_air: Seconds,

	/// The maximum height reached during the jump.
	pub height: f32,

	/// How close to a perfect airpath this jump was.
	///
	/// The closer to 1.0 the better.
	pub airpath: f32,

	/// How far the landing position deviates from the jumpoff position.
	pub deviation: f32,

	/// The average strafe width.
	pub average_width: f32,

	/// The amount of time spent mid-air.
	pub airtime: Seconds,
}

/// Response body for creating a new jumpstat.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedJumpstat {
	/// The jumpstat's ID.
	pub jumpstat_id: JumpstatID,
}
