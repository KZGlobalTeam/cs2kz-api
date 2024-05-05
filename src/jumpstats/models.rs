//! Types used for describing jumpstats.

use chrono::{DateTime, Utc};
use cs2kz::{JumpType, Mode, SteamID};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::id::make_id;
use crate::players::Player;
use crate::servers::ServerInfo;
use crate::time::Seconds;

make_id!(JumpstatID as u64);

/// A jumpstat.
#[derive(Debug, Serialize, ToSchema)]
pub struct Jumpstat {
	/// The jumpstat's ID.
	pub id: JumpstatID,

	/// The jump type.
	#[serde(rename = "type")]
	pub jump_type: JumpType,

	/// The mode this jump was performed in.
	pub mode: Mode,

	/// The player this jump was performed by.
	pub player: Player,

	/// The server this jump was performed on.
	pub server: ServerInfo,

	/// The amount of strafes done in this jump.
	pub strafes: u8,

	/// The jump's distance.
	pub distance: f32,

	/// The % of how much airtime was spent gaining speed.
	pub sync: f32,

	/// The jump's speed at jumpoff.
	pub pre: f32,

	/// The maximum speed during the jump.
	pub max: f32,

	/// The % of how much airtime was spent pressing both directional keys at once.
	pub overlap: f32,

	/// The % of how much airtime keys were pressed but no speed was gained.
	pub bad_angles: f32,

	/// The % of how much airtime was spent not gaining speed.
	pub dead_air: f32,

	/// The maximum height during this jump (in units).
	pub height: f32,

	/// How close to a perfect airpath this jump was.
	///
	/// The closer to 1.0 the better.
	pub airpath: f32,

	/// How far the landing point deviates from the jumpoff point.
	pub deviation: f32,

	/// The average strafe width.
	pub average_width: f32,

	/// How much time the player spent in the air.
	pub airtime: Seconds,

	/// When this jump was submitted.
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

/// Request body for submitting new jumpstats.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct NewJumpstat {
	/// The jump type.
	#[serde(rename = "type")]
	pub jump_type: JumpType,

	/// The mode this jump was performed in.
	pub mode: Mode,

	/// The SteamID of the player who performed this jump.
	pub player_id: SteamID,

	/// The amount of strafes done in this jump.
	pub strafes: u8,

	/// The jump's distance.
	pub distance: f32,

	/// The % of how much airtime was spent gaining speed.
	pub sync: f32,

	/// The jump's speed at jumpoff.
	pub pre: f32,

	/// The maximum speed during the jump.
	pub max: f32,

	/// The % of how much airtime was spent pressing both directional keys at once.
	pub overlap: f32,

	/// The % of how much airtime keys were pressed but no speed was gained.
	pub bad_angles: f32,

	/// The % of how much airtime was spent not gaining speed.
	pub dead_air: f32,

	/// The maximum height during this jump (in units).
	pub height: f32,

	/// How close to a perfect airpath this jump was.
	///
	/// The closer to 1.0 the better.
	pub airpath: f32,

	/// How far the landing point deviates from the jumpoff point.
	pub deviation: f32,

	/// The average strafe width.
	pub average_width: f32,

	/// How much time the player spent in the air.
	pub airtime: Seconds,
}

/// A newly created jumpstat.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedJumpstat {
	/// The jumpstat's ID.
	pub jumpstat_id: JumpstatID,
}
