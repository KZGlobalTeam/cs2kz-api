//! This module holds types related to KZ players.

use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Style, Tier};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

/// Information about a map.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
	"id": 1,
	"workshop_id": 3070194623_u32,
	"name": "kz_checkmate",
	"mappers": [
		{
			"steam_id": "STEAM_1:0:102468802",
			"name": "GameChaos"
		}
	],
	"courses": [
		{
			"id": 1,
			"stage": 0,
			"mappers": [
				{
					"steam_id": "STEAM_1:0:102468802",
					"name": "GameChaos"
				}
			],
			"filters": [
				{
					"mode": "kz_modded",
					"has_teleports": true,
					"tier": 3,
					"ranked": true
				},
				{
					"mode": "kz_modded",
					"has_teleports": false,
					"tier": 4,
					"ranked": true
				}
			]
		}
	],
	"filesize": 190335000,
	"created_on": "2023-12-10T10:41:01Z",
	"updated_on": "2023-12-10T10:41:01Z"
}))]
pub struct KZMap {
	/// The map's ID.
	pub id: u16,

	/// The map's Steam Workshop ID.
	pub workshop_id: u32,

	/// The map's name.
	pub name: String,

	/// List of the players who created this map.
	pub mappers: Vec<Mapper>,

	/// List of courses on this map.
	pub courses: Vec<Course>,

	/// The filesize of this map in bytes.
	pub filesize: u64,

	/// Timestamp of when this map was initially approved.
	pub created_on: DateTime<Utc>,

	/// Timestamp of when this map was last updated.
	pub updated_on: DateTime<Utc>,
}

/// Information about a mapper.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
	"steam_id": "STEAM_1:1:161178172",
	"name": "AlphaKeks"
}))]
pub struct Mapper {
	/// The mapper's SteamID.
	pub steam_id: SteamID,

	/// The mapper's name.
	pub name: String,
}

/// Information about a course.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
	"id": 1,
	"stage": 0,
	"mappers": [
		{
			"steam_id": "STEAM_1:0:102468802",
			"name": "GameChaos"
		}
	],
	"filters": [
		{
			"mode": "kz_modded",
			"has_teleports": true,
			"tier": 3,
			"ranked": true
		},
		{
			"mode": "kz_modded",
			"has_teleports": false,
			"tier": 4,
			"ranked": true
		}
	]
}))]
pub struct Course {
	/// The course's ID.
	pub id: u32,

	/// The stage of the map this course corresponds to.
	pub stage: u8,

	/// List of the players who created this course.
	pub mappers: Vec<Mapper>,

	/// List of filters that apply to this course.
	pub filters: Vec<Filter>,
}

/// Information about a course filter.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
	"mode": "kz_modded",
	"has_teleports": true,
	"tier": 3,
	"ranked": true
}))]
pub struct Filter {
	/// The mode for this filter.
	pub mode: Mode,

	/// Whether this filter applies to runs with teleports.
	pub has_teleports: bool,

	/// The difficulty of the course with this filter.
	#[serde(serialize_with = "Tier::serialize_integer")]
	#[schema(value_type = u8, minimum = 1, maximum = 10)]
	pub tier: Tier,

	/// Whether the course is ranked with this filter.
	pub ranked: bool,
}

impl FromRow<'_, MySqlRow> for KZMap {
	fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;
		let workshop_id = row.try_get("workshop_id")?;
		let name = row.try_get("name")?;
		let filesize = row.try_get("filesize")?;
		let created_on = row.try_get("created_on")?;
		let updated_on = row.try_get("updated_on")?;

		let mapper_steam_id = row.try_get("mapper_steam_id")?;
		let mapper_steam_id =
			SteamID::from_u32(mapper_steam_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let mapper_name = row.try_get("mapper_name")?;
		let mappers = vec![Mapper { steam_id: mapper_steam_id, name: mapper_name }];

		let course_id = row.try_get("course_id")?;
		let course_stage = row.try_get("course_stage")?;
		let course_mapper_steam_id = row.try_get("course_mapper_steam_id")?;
		let course_mapper_steam_id = SteamID::from_u32(course_mapper_steam_id)
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let course_mapper_name = row.try_get("course_mapper_name")?;

		let filter_mode = row
			.try_get::<u8, _>("filter_mode")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let filter_has_teleports = row.try_get("filter_has_teleports")?;
		let filter_tier = row.try_get("filter_tier")?;
		let filter_ranked = row.try_get("filter_ranked")?;

		let courses = vec![Course {
			id: course_id,
			stage: course_stage,
			mappers: vec![Mapper { steam_id: course_mapper_steam_id, name: course_mapper_name }],
			filters: vec![Filter {
				mode: filter_mode,
				has_teleports: filter_has_teleports,
				tier: filter_tier,
				ranked: filter_ranked,
			}],
		}];

		Ok(Self {
			id,
			workshop_id,
			name,
			mappers,
			courses,
			filesize,
			created_on,
			updated_on,
		})
	}
}

/// Combination of a course and filter.
#[derive(Debug, Serialize, ToSchema)]
pub struct CourseWithFilter {
	/// The course's ID.
	pub id: u32,

	/// The ID of the map the course belongs to.
	pub map_id: u16,

	/// The name of the map the course belongs to.
	pub map_name: String,

	/// The stage of the map this course corresponds to.
	pub map_stage: u8,

	/// The filter's mode.
	pub mode: Mode,

	/// The filter's style.
	pub style: Style,

	/// The course's tier with this filter.
	pub tier: Tier,
}
