use {chrono::NaiveTime, cs2kz::SteamID, sqlx::FromRow};

#[derive(Debug, Clone, FromRow)]
pub struct Player {
	#[sqlx(rename = "id", try_from = "u64")]
	pub steam_id: SteamID,
	pub name: String,
	pub is_banned: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct PlayerWithPlaytime {
	#[sqlx(flatten)]
	pub player: Player,
	pub time_active: NaiveTime,
	pub time_spectating: NaiveTime,
	pub time_afk: NaiveTime,
}
