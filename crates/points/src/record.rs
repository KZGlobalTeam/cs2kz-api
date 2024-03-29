use std::cmp;

use chrono::{DateTime, Utc};
use cs2kz::{SteamID, Tier};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
	pub id: u32,
	pub course_id: u32,
	pub map_id: u16,
	pub map_name: String,
	pub map_stage: u8,
	pub stage_tier: Tier,
	pub steam_id: SteamID,
	pub player_name: String,
	pub mode: String,
	pub time: f64,
	pub teleports: u16,
	pub created_on: DateTime<Utc>,
}

impl PartialEq for Record {
	fn eq(&self, other: &Self) -> bool {
		self.id.eq(&other.id)
	}
}

impl Eq for Record {}

impl PartialOrd for Record {
	fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Record {
	fn cmp(&self, other: &Self) -> cmp::Ordering {
		self.time
			.partial_cmp(&other.time)
			.unwrap_or_else(|| self.created_on.cmp(&other.created_on))
	}
}
