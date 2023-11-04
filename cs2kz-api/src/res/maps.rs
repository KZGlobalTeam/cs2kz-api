use {
	chrono::{DateTime, Utc},
	cs2kz::SteamID,
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
	pub owned_by: Mapper,
	pub created_on: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Course {
	pub id: u32,
	pub stage: u8,
	// TODO(AlphaKeks): enum this
	pub difficulty: u8,
	pub created_by: Mapper,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Mapper {
	pub name: String,
	pub steam_id: SteamID,
}
