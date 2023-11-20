use {cs2kz::SteamID, sqlx::FromRow};

#[derive(Debug, Clone, FromRow)]
pub struct Player {
	#[sqlx(try_from = "u64")]
	pub steam_id: SteamID,
	pub name: String,
	pub is_banned: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct PlayerWithPlaytime {
	#[sqlx(flatten)]
	pub player: Player,
	pub time_active: u32,
	pub time_spectating: u32,
	pub time_afk: u32,
	pub perfs: u16,
	pub bhops_tick0: u16,
	pub bhops_tick1: u16,
	pub bhops_tick2: u16,
	pub bhops_tick3: u16,
	pub bhops_tick4: u16,
	pub bhops_tick5: u16,
	pub bhops_tick6: u16,
	pub bhops_tick7: u16,
	pub bhops_tick8: u16,
}
