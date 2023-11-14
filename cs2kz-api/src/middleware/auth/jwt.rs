use {
	chrono::{DateTime, Utc},
	jsonwebtoken as jwt,
	serde::{Deserialize, Serialize},
};

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

	pub fn timestamp(&self) -> DateTime<Utc> {
		DateTime::from_timestamp(self.exp as _, 0).unwrap()
	}
}
