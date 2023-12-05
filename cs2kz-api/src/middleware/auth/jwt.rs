use jsonwebtoken as jwt;
use serde::{Deserialize, Serialize};

const HALF_HOUR: u64 = 60 * 30;

#[derive(Clone, Serialize, Deserialize)]
pub struct GameServerInfo {
	pub id: u16,
	pub exp: u64,
}

impl GameServerInfo {
	pub fn new(id: u16) -> Self {
		Self { id, exp: jwt::get_current_timestamp() + HALF_HOUR }
	}
}
