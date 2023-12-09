use cs2kz::SteamID;
use jsonwebtoken as jwt;
use serde::{Deserialize, Serialize};

use crate::steam;

const HALF_HOUR: u64 = 60 * 30;
const ONE_DAY: u64 = HALF_HOUR * 48;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GameServerToken {
	pub id: u16,
	pub plugin_version: u16,

	#[serde(rename = "exp")]
	pub expires_at: u64,
}

impl GameServerToken {
	pub fn new(id: u16, plugin_version: u16) -> Self {
		Self {
			id,
			plugin_version,
			expires_at: jwt::get_current_timestamp() + HALF_HOUR,
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WebUser {
	pub steam_id: SteamID,

	#[serde(rename = "exp")]
	pub expires_at: u64,
}

impl From<steam::AuthResponse> for WebUser {
	fn from(value: steam::AuthResponse) -> Self {
		Self {
			steam_id: value.steam_id,
			expires_at: jwt::get_current_timestamp() + ONE_DAY,
		}
	}
}
