use {
	cs2kz::SteamID,
	serde::Serialize,
	sqlx::FromRow,
	utoipa::{ToResponse, ToSchema},
};

pub mod player;
pub mod bans;
pub mod maps;
pub mod servers;
pub mod records;

/// Information about a player.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct PlayerInfo {
	/// The player's Steam name.
	pub name: String,

	/// The player's `SteamID`.
	pub steam_id: SteamID,
}

#[derive(ToResponse)]
#[response(description = "Request body is malformed in some way.")]
pub struct BadRequest;
