use std::net::Ipv4Addr;

use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Player {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's latest known name.
	pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FullPlayer {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's latest known name.
	pub name: String,

	/// Whether this player is currently banned.
	pub is_banned: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewPlayer {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's current name.
	pub name: String,

	/// The player's current IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,
}
