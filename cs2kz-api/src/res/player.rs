use {crate::database, chrono::NaiveTime, cs2kz::SteamID, serde::Serialize, utoipa::ToSchema};

/// A KZ player.
#[derive(Debug, Serialize, ToSchema)]
pub struct Player {
	/// The player's Steam name.
	pub name: String,

	/// The player's `SteamID`.
	pub steam_id: SteamID,

	/// The player's total active time spent on verified servers.
	pub playtime: NaiveTime,

	/// The player's total AFK time spent on verified servers.
	pub afktime: NaiveTime,

	/// Whether the player is banned.
	pub is_banned: bool,
}

impl From<database::PlayerWithPlaytime> for Player {
	fn from(row: database::PlayerWithPlaytime) -> Self {
		Self {
			name: row.player.name,
			steam_id: row.player.steam_id,
			playtime: row.playtime,
			afktime: row.afktime,
			is_banned: row.player.is_banned,
		}
	}
}
