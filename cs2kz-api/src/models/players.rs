//! This module holds types related to KZ players.

use cs2kz::SteamID;
use serde::Serialize;
use utoipa::ToSchema;

/// Information about a player.
#[derive(Debug, PartialEq, Eq, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[schema(example = json!({
  "steam_id": "STEAM_1:1:161178172",
  "name": "AlphaKeks"
}))]
pub struct Player {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's name.
	pub name: String,
}
