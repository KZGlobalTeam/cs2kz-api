use chrono::{DateTime, Utc};
use cs2kz::{Mode, Runtype, SteamID, Tier};
use serde::Serialize;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use super::PlayerInfo;

/// A KZ map.
#[derive(Debug, Serialize, ToSchema)]
pub struct KZMap {
	/// The map's ID.
	pub id: u16,

	/// The map's name.
	pub name: String,

	/// The map's Steam workshop ID.
	pub workshop_id: u32,

	/// The filesize of the map.
	pub filesize: u64,

	/// The players who created this map.
	pub mappers: Vec<PlayerInfo>,

	/// A list of the courses on this map.
	pub courses: Vec<MapCourse>,

	/// Timestamp of when this map was globalled.
	pub created_on: DateTime<Utc>,
}

/// A course on a KZ map.
#[derive(Debug, Serialize, ToSchema)]
pub struct MapCourse {
	/// The ID of the course.
	pub id: u32,

	/// The stage this course corresponds to.
	pub stage: u8,

	/// The players who created this course.
	pub mappers: Vec<PlayerInfo>,

	/// List of filters.
	pub filters: Vec<CourseFilter>,
}

/// A filter for a course on a KZ map.
///
/// This determines which (mode, runtype) combination is feasible on a given course, as well as its
/// ranked status and difficulty.
#[derive(Debug, Serialize, ToSchema)]
pub struct CourseFilter {
	/// The ID of this filter.
	pub id: u32,

	/// The mode which the filter applies to.
	pub mode: Mode,

	/// The runtype which the filter applies to.
	pub runtype: Runtype,

	/// The difficulty of this course with the given (mode, runtype) combination.
	pub tier: Tier,

	/// Whether this course with the given (mode, runtype) combination is ranked.
	pub ranked: bool,
}

impl FromRow<'_, MySqlRow> for KZMap {
	fn from_row(row: &MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;
		let name = row.try_get("name")?;
		let workshop_id = row.try_get("workshop_id")?;
		let filesize = row.try_get("filesize")?;
		let created_on = row.try_get("created_on")?;

		let mapper_name = row.try_get("mapper_name")?;
		let mapper_steam_id = row.try_get("mapper_steam_id")?;
		let mapper_steam_id = SteamID::from_id32(mapper_steam_id)
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let mappers = vec![PlayerInfo { name: mapper_name, steam_id: mapper_steam_id }];

		let course_id = row.try_get("course_id")?;
		let course_stage = row.try_get("course_stage")?;
		let course_mapper_name = row.try_get("course_mapper_name")?;
		let course_mapper_steam_id = row.try_get("course_mapper_steam_id")?;
		let course_mapper_steam_id = SteamID::from_id32(course_mapper_steam_id)
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let course_mappers =
			vec![PlayerInfo { name: course_mapper_name, steam_id: course_mapper_steam_id }];

		let filter_id = row.try_get("filter_id")?;
		let filter_mode: u8 = row.try_get("filter_mode")?;
		let filter_mode =
			Mode::try_from(filter_mode).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let filter_has_teleports: bool = row.try_get("filter_has_teleports")?;
		let filter_runtype = Runtype::from(filter_has_teleports);
		let filter_tier: u8 = row.try_get("filter_tier")?;
		let filter_tier =
			Tier::try_from(filter_tier).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let filter_ranked = row.try_get("filter_ranked")?;

		let course_filters = vec![CourseFilter {
			id: filter_id,
			mode: filter_mode,
			runtype: filter_runtype,
			tier: filter_tier,
			ranked: filter_ranked,
		}];

		let courses = vec![MapCourse {
			id: course_id,
			stage: course_stage,
			mappers: course_mappers,
			filters: course_filters,
		}];

		Ok(Self { id, name, workshop_id, filesize, mappers, courses, created_on })
	}
}
