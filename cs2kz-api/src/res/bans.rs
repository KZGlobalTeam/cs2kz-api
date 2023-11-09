use {
	chrono::{DateTime, Utc},
	cs2kz::SteamID,
	serde::Serialize,
	sqlx::FromRow,
	utoipa::ToSchema,
};

/// Information about a ban of a player.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Ban {
	/// The ban's ID.
	pub id: u32,

	/// The player's Steam name.
	pub name: String,

	/// The player's `SteamID`.
	#[sqlx(try_from = "u64")]
	pub steam_id: SteamID,

	// TODO(AlphaKeks): enum this
	/// The reason for the ban.
	pub reason: String,

	/// Timestamp of when the player was banned.
	pub date: DateTime<Utc>,
}
