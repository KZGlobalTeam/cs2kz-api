use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Tier};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::database::{GlobalStatus, RankedStatus};
use crate::players::Player;

/// A KZ map.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KZMap {
	/// The map's ID.
	pub id: u16,

	/// The map's Steam Workshop ID.
	pub workshop_id: u32,

	/// The map's name.
	pub name: String,

	/// List of players who have contributed to creating this map.
	pub mappers: Vec<Player>,

	/// List of [`Course`]s which are part of this map.
	pub courses: Vec<Course>,

	/// The current [global status] of the map.
	///
	/// [global status]: GlobalStatus
	pub global_status: GlobalStatus,

	/// The map's unique checksum.
	///
	/// This is calculated by running the map's `.vpk` file through [crc32].
	///
	/// [crc32]: https://en.wikipedia.org/wiki/Cyclic_redundancy_check
	pub checksum: u32,

	/// When this map was approved for globalling.
	pub created_on: DateTime<Utc>,
}

impl KZMap {
	/// Flattens a list of maps containing overlaps. See [`KZMap::reduce()`] for more details.
	pub fn flatten(maps: Vec<Self>) -> Vec<Self> {
		maps.into_iter()
			.group_by(|map| map.id)
			.into_iter()
			.filter_map(|(_, maps)| maps.reduce(Self::reduce))
			.collect_vec()
	}

	/// Combines two maps into one, aggregating common mappers and courses.
	/// Used for database queries, since SQL does not like nested data.
	pub fn reduce(mut self, other: Self) -> Self {
		for mapper in other.mappers {
			if !self.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
				self.mappers.push(mapper);
			}
		}

		for course in other.courses {
			let Some(c) = self.courses.iter_mut().find(|c| c.id == course.id) else {
				self.courses.push(course);
				continue;
			};

			for mapper in course.mappers {
				if !c.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
					c.mappers.push(mapper);
				}
			}

			for filter in course.filters {
				if !c.filters.iter().any(|f| f == &filter) {
					c.filters.push(filter);
				}
			}
		}

		self
	}
}

impl FromRow<'_, MySqlRow> for KZMap {
	fn from_row(row: &'_ MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;
		let workshop_id = row.try_get("workshop_id")?;
		let name = row.try_get("name")?;
		let global_status = row.try_get("global_status")?;
		let checksum = row.try_get("checksum")?;
		let created_on = row.try_get("created_on")?;

		let mappers = vec![Player {
			steam_id: row.try_get("mapper_steam_id")?,
			name: row.try_get("mapper_name")?,
			is_banned: row.try_get("mapper_is_banned")?,
		}];

		let courses = vec![Course {
			id: row.try_get("course_id")?,
			stage: row.try_get("course_stage")?,
			mappers: vec![Player {
				steam_id: row.try_get("course_mapper_steam_id")?,
				name: row.try_get("course_mapper_name")?,
				is_banned: row.try_get("course_mapper_is_banned")?,
			}],
			filters: vec![Filter {
				id: row.try_get("filter_id")?,
				mode: row.try_get("filter_mode")?,
				teleports: row.try_get("filter_teleports")?,
				tier: row.try_get("filter_tier")?,
				ranked_status: row.try_get("filter_ranked_status")?,
			}],
		}];

		Ok(Self {
			id,
			workshop_id,
			name,
			mappers,
			courses,
			global_status,
			checksum,
			created_on,
		})
	}
}

/// A map course.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Course {
	/// The course's ID.
	pub id: u32,

	/// The course's stage.
	pub stage: u8,

	/// List of the players who have contributed to creating this course.
	pub mappers: Vec<Player>,

	/// List of filters that apply to this course.
	pub filters: Vec<Filter>,
}

/// A course filter.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Filter {
	/// The filter's ID.
	pub id: u32,

	/// The mode this filter is associated with.
	pub mode: Mode,

	/// Whether this filter is for standard or pro runs.
	pub teleports: bool,

	/// The [`Tier`] of this filter.
	pub tier: Tier,

	/// The [ranked status] of this filter.
	///
	/// [ranked status]: RankedStatus
	pub ranked_status: RankedStatus,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewMap {
	/// The map's workshop ID.
	pub workshop_id: u32,

	/// The initial [global status] of this map.
	///
	/// [global status]: GlobalStatus
	pub global_status: GlobalStatus,

	/// List of players who have contributed to creating this map.
	pub mappers: Vec<SteamID>,

	/// List of courses.
	pub courses: Vec<NewCourse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewCourse {
	/// The course's stage.
	pub stage: u8,

	/// The course's name.
	pub name: Option<String>,

	/// List of players who have contributed to creating this course.
	pub mappers: Vec<SteamID>,

	/// List of filters for this course.
	pub filters: [NewFilter; 4],
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewFilter {
	/// The mode this filter is associated with.
	pub mode: Mode,

	/// Whether this filter is for standard or pro runs.
	pub teleports: bool,

	/// The [`Tier`] of this filter.
	pub tier: Tier,

	/// The [ranked status] of this filter.
	///
	/// [ranked status]: RankedStatus
	pub ranked_status: RankedStatus,
}

/// A newly created [`KZMap`].
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedMap {
	/// The map's ID.
	pub map_id: u16,
}

/// An update to a [`KZMap`].
#[derive(Debug, Deserialize, ToSchema)]
pub struct MapUpdate {
	/// A new [global status].
	///
	/// [global status]: GlobalStatus
	pub global_status: Option<GlobalStatus>,

	/// A new workshop ID.
	pub workshop_id: Option<u32>,

	/// Fetch the latest version of the map from Steam and update the name.
	#[serde(default)]
	pub name: bool,

	/// Fetch the latest version of the map from Steam and update the checksum.
	#[serde(default)]
	pub checksum: bool,

	/// List of mappers to add.
	pub added_mappers: Option<Vec<SteamID>>,

	/// List of mappers to remove.
	pub removed_mappers: Option<Vec<SteamID>>,

	/// List of course updates.
	pub course_updates: Option<Vec<CourseUpdate>>,
}

/// An update to a [`Course`].
#[derive(Debug, Deserialize, ToSchema)]
pub struct CourseUpdate {
	/// The course's [`id`].
	///
	/// [`id`]: Course::id
	pub id: u32,

	/// List of mappers to add.
	pub added_mappers: Option<Vec<SteamID>>,

	/// List of mappers to remove.
	pub removed_mappers: Option<Vec<SteamID>>,

	/// List of updates to [`Filter`]s.
	pub filter_updates: Option<Vec<FilterUpdate>>,
}

/// An update to a [`Filter`].
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct FilterUpdate {
	/// The filter's [`id`].
	///
	/// [`id`]: Filter::id
	pub id: u32,

	/// A new [`Tier`].
	pub tier: Option<Tier>,

	/// A new [ranked status].
	///
	/// [ranked status]: RankedStatus
	pub ranked_status: Option<RankedStatus>,
}
