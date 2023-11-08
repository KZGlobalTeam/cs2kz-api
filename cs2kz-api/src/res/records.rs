use {
	chrono::{DateTime, Utc},
	cs2kz::{Mode, Runtype, SteamID, Style, Tier},
	serde::Serialize,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
	pub id: u64,
	pub map: RecordMap,
	pub course: RecordCourse,
	pub mode: Mode,
	pub runtype: Runtype,
	pub style: Style,
	pub player: RecordPlayer,
	pub server: RecordServer,
	pub teleports: u16,
	pub time: f64,
	pub created_on: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordMap {
	pub id: u16,
	pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordCourse {
	pub id: u32,
	pub stage: u8,
	pub tier: Tier,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordPlayer {
	pub steam_id: SteamID,
	pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordServer {
	pub id: u16,
	pub name: String,
}
