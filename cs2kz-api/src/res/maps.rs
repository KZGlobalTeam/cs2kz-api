use {
	super::PlayerInfo,
	chrono::{DateTime, Utc},
	cs2kz::Tier,
	serde::Serialize,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct KZMap {
	pub id: u16,
	pub name: String,
	pub workshop_id: u32,
	pub courses: Vec<Course>,
	pub filesize: u64,
	pub owned_by: PlayerInfo,
	pub created_on: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Course {
	pub id: u32,
	pub stage: u8,
	pub tier: Tier,
	pub created_by: PlayerInfo,
}
