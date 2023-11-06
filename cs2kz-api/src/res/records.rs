use {
	chrono::{DateTime, Utc},
	cs2kz::{Mode, Runtype, SteamID, Style, Tier},
	serde::Serialize,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
	pub id: u32,
	pub map_id: u16,
	pub map_name: String,
	pub map_stage: u8,
	pub course_id: u32,
	pub course_tier: Tier,
	pub mode: Mode,
	pub runtype: Runtype,
	pub style: Style,
	pub player_name: String,
	pub steam_id: SteamID,
	pub server_id: u16,
	pub server_name: String,
	pub teleports: u16,
	pub time: f64,
	pub created_on: DateTime<Utc>,
}
