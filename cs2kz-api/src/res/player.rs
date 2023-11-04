use {crate::database, chrono::NaiveTime, cs2kz::SteamID, serde::Serialize, utoipa::ToSchema};

#[derive(Debug, Serialize, ToSchema)]
pub struct Player {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's Steam name.
	pub name: String,

	/// The player's total active time spent on verified servers.
	pub playtime: NaiveTime,

	/// The player's total AFK time spent on verified servers.
	pub afktime: NaiveTime,

	/// Whether the player is banned.
	pub is_banned: bool,
}

impl Player {
	pub const fn new(
		name: String,
		steam_id: SteamID,
		playtime: NaiveTime,
		afktime: NaiveTime,
		is_banned: bool,
	) -> Self {
		Self { name, steam_id, playtime, afktime, is_banned }
	}
}

impl From<database::PlayerWithPlaytime> for Player {
	fn from(row: database::PlayerWithPlaytime) -> Self {
		Self::new(
			row.player.name,
			row.player.steam_id,
			row.playtime,
			row.afktime,
			row.player.is_banned,
		)
	}
}
