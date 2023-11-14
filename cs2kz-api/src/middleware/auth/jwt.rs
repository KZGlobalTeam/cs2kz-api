use {
	chrono::{DateTime, Utc},
	serde::{Deserialize, Serialize},
	std::time::Duration,
};

const HALF_HOUR: Duration = Duration::from_secs(60 * 30);

#[derive(Clone, Serialize, Deserialize)]
pub struct GameServerInfo {
	pub id: u16,
	pub expires_on: DateTime<Utc>,
}

impl GameServerInfo {
	pub fn new(id: u16) -> Self {
		Self { id, expires_on: Utc::now() + HALF_HOUR }
	}
}
