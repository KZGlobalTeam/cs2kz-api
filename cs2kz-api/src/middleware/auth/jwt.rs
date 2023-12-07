use jsonwebtoken as jwt;
use serde::{Deserialize, Serialize};

const HALF_HOUR: u64 = 60 * 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
