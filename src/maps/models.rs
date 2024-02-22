use std::cmp;
use std::num::{NonZeroU32, NonZeroU8};

use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Tier};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
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
	#[schema(value_type = u32, minimum = 1)]
	pub workshop_id: NonZeroU32,

	/// The map's name.
	pub name: String,

	/// List of players who have contributed to creating this map.
	pub mappers: Vec<Player>,

	/// List of courses which are part of this map.
	pub courses: Vec<Course>,

	/// The current global status of the map.
	pub global_status: GlobalStatus,

	/// The map's description.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

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
	/// Groups any maps with the same ID and reduces them into a single value.
	///
	/// See [`KZMap::reduce()`].
	pub fn flatten(maps: Vec<Self>) -> Vec<Self> {
		maps.into_iter()
			.group_by(|map| map.id)
			.into_iter()
			.filter_map(|(_, maps)| maps.reduce(Self::reduce))
			.collect_vec()
	}

	/// Combines two maps into one, aggregating common mappers and courses.
	pub fn reduce(mut self, other: Self) -> Self {
		assert_eq!(self.id, other.id);

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
		let id = crate::sqlx::non_zero!("id" as u16, row)?;
		let workshop_id = crate::sqlx::non_zero!("workshop_id" as u32, row)?;
		let name = row.try_get("name")?;
		let global_status = row.try_get("global_status")?;
		let description = row.try_get("description").ok();
		let checksum = row.try_get("checksum")?;
		let created_on = row.try_get("created_on")?;

		let mappers = vec![Player {
			steam_id: row.try_get("mapper_steam_id")?,
			name: row.try_get("mapper_name")?,
		}];

		let courses = vec![Course {
			id: crate::sqlx::non_zero!("course_id" as u32, row)?,
			name: row.try_get("course_name")?,
			description: row.try_get("course_description").ok(),
			stage: crate::sqlx::non_zero!("course_stage" as u8, row)?,
			mappers: vec![Player {
				steam_id: row.try_get("course_mapper_steam_id")?,
				name: row.try_get("course_mapper_name")?,
			}],
			filters: vec![Filter {
				id: crate::sqlx::non_zero!("filter_id" as u32, row)?,
				mode: row.try_get("filter_mode")?,
				teleports: row.try_get("filter_teleports")?,
				tier: row.try_get("filter_tier")?,
				ranked_status: row.try_get("filter_ranked_status")?,
				notes: row.try_get("filter_notes").ok(),
			}],
		}];

		Ok(Self {
			id,
			workshop_id,
			name,
			mappers,
			courses,
			global_status,
			description,
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

	/// The course's name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// The course's description.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// The course's stage.
	#[schema(value_type = u8, minimum = 1)]
	pub stage: NonZeroU8,

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

	/// The tier of this filter.
	pub tier: Tier,

	/// The ranked status of this filter.
	pub ranked_status: RankedStatus,

	/// Notes about this filter.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub notes: Option<String>,
}

/// The request body for creating a new map.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewMap {
	/// The map's workshop ID.
	#[schema(value_type = u32, minimum = 1)]
	pub workshop_id: NonZeroU32,

	/// The initial global status of this map.
	pub global_status: GlobalStatus,

	/// Description of the map.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub description: Option<String>,

	/// List of players who have contributed to creating this map.
	#[serde(deserialize_with = "crate::serde::deserialize_non_empty_vec")]
	pub mappers: Vec<SteamID>,

	/// List of courses.
	#[serde(deserialize_with = "NewMap::deserialize_courses")]
	pub courses: Vec<NewCourse>,
}

impl NewMap {
	/// Custom deserialization logic for a new map's courses.
	///
	/// This will enforce the following invariants:
	///   - [`NewMap::courses`] is not empty
	///   - [`NewMap::courses`] is sorted by [`NewCourse::stage`]
	///   - [`NewMap::courses`] is contiguous by [`NewCourse::stage`]
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<Vec<NewCourse>, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error as E;

		let mut courses: Vec<NewCourse> = crate::serde::deserialize_non_empty_vec(deserializer)?;

		courses.sort_by_key(|course| course.stage);

		let are_contiguous = courses.windows(2).all(|courses| match courses.get(1) {
			None => true,
			Some(course) if course.stage.get() == courses[0].stage.get() + 1 => true,
			Some(_) => false,
		});

		if !are_contiguous {
			return Err(E::custom("course stages are not contiguous"));
		}

		Ok(courses)
	}
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewCourse {
	/// The course's stage.
	#[schema(value_type = u8, minimum = 1)]
	pub stage: NonZeroU8,

	/// The course's name.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub name: Option<String>,

	/// Description of the course.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub description: Option<String>,

	/// List of players who have contributed to creating this course.
	#[serde(deserialize_with = "crate::serde::deserialize_non_empty_vec")]
	pub mappers: Vec<SteamID>,

	/// List of filters for this course.
	#[serde(deserialize_with = "NewCourse::validate_filters")]
	pub filters: [NewFilter; 4],
}

impl NewCourse {
	/// Custom deserialization logic for a new course's filters.
	///
	/// This will enforce the following invariants:
	///   - There are exactly 4 filters
	///   - [`NewCourse::filters`] is sorted
	///   - All 4 permutations of (mode, teleports) are covered
	///   - Any filters with a tier higher than [`Tier::Death`] cannot also be marked as
	///     [`RankedStatus::Ranked`]
	fn validate_filters<'de, D>(deserializer: D) -> Result<[NewFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error as E;

		let mut filters = <[NewFilter; 4]>::deserialize(deserializer)?;

		filters.sort_by_key(|filter| (filter.mode, cmp::Reverse(filter.teleports)));

		const EXPECTED: [(Mode, bool); 4] = [
			(Mode::Vanilla, true),
			(Mode::Vanilla, false),
			(Mode::Classic, true),
			(Mode::Classic, false),
		];

		if let Some((_, (mode, teleports))) = filters
			.iter()
			.map(|filter| (filter.mode, filter.teleports))
			.zip(EXPECTED)
			.find(|(a, b)| a != b)
		{
			return Err(E::custom(format_args!(
				"filter for ({mode}, {runtype}) is missing",
				runtype = match teleports {
					true => "TP",
					false => "Pro",
				},
			)));
		}

		if let Some(tier) = filters
			.iter()
			.find(|filter| {
				filter.tier > Tier::Death && filter.ranked_status == RankedStatus::Ranked
			})
			.map(|filter| filter.tier)
		{
			return Err(E::custom(format_args!(
				"tier `{tier}` is too high for a ranked filter"
			)));
		}

		Ok(filters)
	}
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewFilter {
	/// The mode this filter is associated with.
	pub mode: Mode,

	/// Whether this filter is for standard or pro runs.
	pub teleports: bool,

	/// The tier of this filter.
	pub tier: Tier,

	/// The ranked status of this filter.
	pub ranked_status: RankedStatus,

	/// Notes about the filter.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub notes: Option<String>,
}

/// Response body for newly created maps.
///
/// See [`NewMap`].
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedMap {
	/// The map's ID.
	pub map_id: u16,
}

/// Request body for updates to a map.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct MapUpdate {
	/// A new global status.
	pub global_status: Option<GlobalStatus>,

	/// New description for the map.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub description: Option<String>,

	/// A new workshop ID.
	pub workshop_id: Option<NonZeroU32>,

	/// Fetch the latest version of the map from Steam and update its name and checksum.
	#[serde(default)]
	pub check_steam: bool,

	/// List of mappers to add.
	#[serde(default)]
	pub added_mappers: Vec<SteamID>,

	/// List of mappers to remove.
	#[serde(default)]
	pub removed_mappers: Vec<SteamID>,

	/// List of course updates.
	#[serde(default)]
	pub course_updates: Vec<CourseUpdate>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CourseUpdate {
	/// The course's ID.
	pub id: u32,

	/// New name for the course.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub name: Option<String>,

	/// New description for the course.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub description: Option<String>,

	/// List of mappers to add.
	#[serde(default)]
	pub added_mappers: Vec<SteamID>,

	/// List of mappers to remove.
	#[serde(default)]
	pub removed_mappers: Vec<SteamID>,

	/// List of updates to filters.
	#[serde(default)]
	pub filter_updates: Vec<FilterUpdate>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FilterUpdate {
	/// The filter's ID.
	pub id: u32,

	/// A new tier.
	pub tier: Option<Tier>,

	/// A new ranked status.
	pub ranked_status: Option<RankedStatus>,

	/// New notes for the course.
	#[serde(deserialize_with = "crate::serde::deserialize_empty_string_as_none")]
	pub notes: Option<String>,
}
