use cs2kz::SteamID;
use serde::Serialize;
use utoipa::ToSchema;

use crate::database;

/// A KZ player.
#[derive(Debug, Serialize, ToSchema)]
pub struct Player {
	/// The player's Steam name.
	pub name: String,

	/// The player's `SteamID`.
	pub steam_id: SteamID,

	/// The player's total active time spent on verified servers.
	pub time_active: u32,

	/// The player's total time spent spectating on verified servers.
	pub time_spectating: u32,

	/// The player's total AFK time spent on verified servers.
	pub time_afk: u32,

	/// Whether the player is banned.
	pub is_banned: bool,

	/// How many perfect bhops the player has hit in total.
	pub perfs: u16,

	/// How many bhops the player has hit 0 ticks after landing.
	pub bhops_tick0: u16,

	/// How many bhops the player has hit 1 ticks after landing.
	pub bhops_tick1: u16,

	/// How many bhops the player has hit 2 ticks after landing.
	pub bhops_tick2: u16,

	/// How many bhops the player has hit 3 ticks after landing.
	pub bhops_tick3: u16,

	/// How many bhops the player has hit 4 ticks after landing.
	pub bhops_tick4: u16,

	/// How many bhops the player has hit 5 ticks after landing.
	pub bhops_tick5: u16,

	/// How many bhops the player has hit 6 ticks after landing.
	pub bhops_tick6: u16,

	/// How many bhops the player has hit 7 ticks after landing.
	pub bhops_tick7: u16,

	/// How many bhops the player has hit 8 ticks after landing.
	pub bhops_tick8: u16,
}

impl From<database::PlayerWithPlaytime> for Player {
	fn from(row: database::PlayerWithPlaytime) -> Self {
		Self {
			name: row.player.name,
			steam_id: row.player.steam_id,
			time_active: row.time_active,
			time_spectating: row.time_spectating,
			time_afk: row.time_afk,
			is_banned: row.player.is_banned,
			perfs: row.perfs,
			bhops_tick0: row.bhops_tick0,
			bhops_tick1: row.bhops_tick1,
			bhops_tick2: row.bhops_tick2,
			bhops_tick3: row.bhops_tick3,
			bhops_tick4: row.bhops_tick4,
			bhops_tick5: row.bhops_tick5,
			bhops_tick6: row.bhops_tick6,
			bhops_tick7: row.bhops_tick7,
			bhops_tick8: row.bhops_tick8,
		}
	}
}
