use {
	chrono::{DateTime, Utc},
	cs2kz::SteamID,
	serde::Serialize,
	sqlx::FromRow,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Ban {
	/// The ban's ID.
	///
	/// Can be used to retrieve its replay.
	pub id: u32,

	/// The player's SteamID.
	#[sqlx(try_from = "u64")]
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
	pub const fn new(
		id: u32,
		steam_id: SteamID,
		name: String,
		reason: String,
		date: DateTime<Utc>,
	) -> Ban {
		Ban { id, steam_id, name, reason, date }
	}
}
