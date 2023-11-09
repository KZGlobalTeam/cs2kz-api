use {
	chrono::{DateTime, Utc},
	cs2kz::SteamID,
	serde::{Deserialize, Serialize},
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

	/// The reason for the ban.
	pub reason: BanReason,

	/// Timestamp of when the player was banned.
	pub date: DateTime<Utc>,
}

/// Reasons for a ban.
#[derive(Debug, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum BanReason {
	AutoBhop,
}
