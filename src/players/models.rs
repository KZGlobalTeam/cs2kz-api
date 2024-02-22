use std::net::Ipv4Addr;

use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Basic information about a KZ player.
///
/// This is included as a field inside many other types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Player {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's latest known name.
	pub name: String,
}

/// Response body for fetching players.
///
/// The [`is_banned`] field is usually not necessary, except in `/players` responses.
///
/// [`is_banned`]: FullPlayer::is_banned
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FullPlayer {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's latest known name.
	pub name: String,

	/// Whether this player is currently banned.
	pub is_banned: bool,
}

/// Request body for registering new KZ players.
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
