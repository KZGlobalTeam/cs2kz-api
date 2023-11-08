use {
	super::PlayerInfo,
	chrono::{DateTime, Utc},
	cs2kz::{SteamID, Tier},
	serde::Serialize,
	sqlx::{mysql::MySqlRow, FromRow, Row},
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct KZMap {
	pub id: u16,
	pub name: String,
	pub workshop_id: u32,
	pub courses: Vec<MapCourse>,
	pub filesize: u64,
	pub owned_by: PlayerInfo,
	pub created_on: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MapCourse {
	pub id: u32,
	pub stage: u8,
	pub tier: Tier,
	pub created_by: PlayerInfo,
}

impl FromRow<'_, MySqlRow> for KZMap {
	fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;
		let name = row.try_get("name")?;
		let workshop_id = row.try_get("workshop_id")?;
		let filesize = row.try_get("filesize")?;
		let created_on = row.try_get("created_on")?;
		let player_name = row.try_get("owner_name")?;
		let steam32_id = row.try_get("owner_id")?;
		let steam_id =
			SteamID::from_id32(steam32_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let owned_by = PlayerInfo { name: player_name, steam_id };

		let course_id = row.try_get("course_id")?;
		let course_stage = row.try_get("course_stage")?;
		let course_tier = row
			.try_get::<u8, _>("course_tier")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let course_created_by_name = row.try_get("course_created_by_name")?;
		let course_created_by_id = row.try_get("course_created_by_id")?;
		let course_created_by_steam_id = SteamID::from_id32(course_created_by_id)
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let courses = vec![MapCourse {
			id: course_id,
			stage: course_stage,
			tier: course_tier,
			created_by: PlayerInfo {
				name: course_created_by_name,
				steam_id: course_created_by_steam_id,
			},
		}];

		Ok(Self { id, name, workshop_id, courses, filesize, owned_by, created_on })
	}
}
