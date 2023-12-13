//! This module holds various types that are encoded into JWTs.

use cs2kz::SteamID;
use serde::{Deserialize, Serialize};

/// Information about a server.
///
/// This struct will be turned into a JWT and given to servers so they can authenticate any
/// requests they make.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ServerClaims {
	/// The server's ID.
	pub id: u16,

	/// The CS2KZ version the server is running on.
	pub plugin_version: u16,

	/// Timestamp of when this token expires.
	#[serde(rename = "exp")]
	pub expires_at: u64,
}

impl ServerClaims {
	/// Constructs a new token. It will expire after 30 minutes.
	pub fn new(id: u16, plugin_version: u16) -> Self {
		Self {
			id,
			plugin_version,
			expires_at: jwt::get_current_timestamp() + (60 * 30),
		}
	}
}

/// Information about a user.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UserClaims {
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// Timestamp of when this token expires.
	#[serde(rename = "exp")]
	pub expires_at: u64,
}

impl UserClaims {
	/// Constructs a new token. It will expire after 1 day.
	pub fn new(steam_id: SteamID) -> Self {
		Self { steam_id, expires_at: jwt::get_current_timestamp() + (60 * 60 * 24) }
	}
}
