use {
	chrono::{DateTime, Utc},
	cs2kz::SteamID,
	serde::Serialize,
	sqlx::FromRow,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Ban {
	/// The player's SteamID.
	#[sqlx(rename = "id", try_from = "u64")]
	pub steam_id: SteamID,

	/// The player's Steam name.
	pub name: String,

	// TODO(AlphaKeks): enum this?
	/// The reason for the ban.
	pub reason: String,

	/// The player's total AFK time spent on verified servers.
	pub date: DateTime<Utc>,
}

impl Ban {
	pub const fn new(steam_id: SteamID, name: String, reason: String, date: DateTime<Utc>) -> Ban {
		Ban { steam_id, name, reason, date }
	}
}
