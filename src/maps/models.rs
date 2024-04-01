//! Types used for describing maps and related concepts.

use std::collections::{BTreeMap, HashSet};
use std::iter;
use std::num::{NonZeroU16, NonZeroU32};

use chrono::{DateTime, Utc};
use cs2kz::{GlobalStatus, Mode, RankedStatus, SteamID, Tier};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::sqlx::query;

/// A KZ map.
///
/// The only reason this is named `FullMap` instead of just `Map`, is because `utoipa` macros are
/// stupid.
#[derive(Debug, Serialize, ToSchema)]
pub struct FullMap {
	/// The map's ID.
	#[schema(value_type = u16)]
	pub id: NonZeroU16,

	/// The map's name.
	pub name: String,

	/// The map's description.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// The map's global status.
	pub global_status: GlobalStatus,

	/// The map's workshop ID.
	pub workshop_id: u32,

	/// The map's checksum.
	pub checksum: u32,

	/// List of players who have contributed to the creation of this map.
	pub mappers: Vec<Player>,

	/// List of courses on this map.
	pub courses: Vec<Course>,

	/// When this map was approved.
	pub created_on: DateTime<Utc>,
}

impl FullMap {
	/// Combines two [`FullMap`]s into one, aggregating [mappers] and [courses].
	///
	/// [mappers]: FullMap::mappers
	/// [courses]: FullMap::courses
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
				if !c.filters.iter().any(|f| f.id == filter.id) {
					c.filters.push(filter);
				}
			}
		}

		self
	}

	/// Groups maps by their ID and flattens them into unique entries.
	pub fn flatten(maps: impl IntoIterator<Item = Self>, limit: usize) -> Vec<Self> {
		maps.into_iter()
			.group_by(|map| map.id)
			.into_iter()
			.filter_map(|(_, maps)| maps.reduce(Self::reduce))
			.take(limit)
			.collect()
	}
}

impl FromRow<'_, MySqlRow> for FullMap {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: query::non_zero!("id" as NonZeroU16, row)?,
			name: row.try_get("name")?,
			description: row.try_get("description")?,
			global_status: row.try_get("global_status")?,
			workshop_id: row.try_get("workshop_id")?,
			checksum: row.try_get("checksum")?,
			mappers: vec![Player {
				name: row.try_get("mapper_name")?,
				steam_id: row.try_get("mapper_id")?,
			}],
			courses: vec![Course::from_row(row)?],
			created_on: row.try_get("created_on")?,
		})
	}
}

/// A map course.
#[derive(Debug, Serialize, ToSchema)]
pub struct Course {
	/// The course's ID.
	#[schema(value_type = u32)]
	pub id: NonZeroU32,

	/// The course's name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// The course's description.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// List of players who have contributed to the creation of this course.
	pub mappers: Vec<Player>,

	/// The course's filters.
	pub filters: Vec<Filter>,
}

impl FromRow<'_, MySqlRow> for Course {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: query::non_zero!("course_id" as NonZeroU32, row)?,
			name: row.try_get("course_name")?,
			description: row.try_get("course_description")?,
			mappers: vec![Player {
				name: row.try_get("course_mapper_name")?,
				steam_id: row.try_get("course_mapper_id")?,
			}],
			filters: vec![Filter {
				id: query::non_zero!("filter_id" as NonZeroU32, row)?,
				mode: row.try_get("filter_mode")?,
				teleports: row.try_get("filter_teleports")?,
				tier: row.try_get("filter_tier")?,
				ranked_status: row.try_get("filter_ranked_status")?,
				notes: row.try_get("filter_notes")?,
			}],
		})
	}
}

/// A course filter.
#[derive(Debug, Serialize, ToSchema)]
pub struct Filter {
	/// The filter's ID.
	#[schema(value_type = u32)]
	pub id: NonZeroU32,

	/// The mode this filter applies to.
	pub mode: Mode,

	/// The "runtype" this filter applies to (whether teleports are used or not).
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Extra notes about this filter.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub notes: Option<String>,
}

/// Request body for submitting new maps.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewMap {
	/// The map's workshop ID.
	pub workshop_id: u32,

	/// The map's description.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// The map's initial global status.
	pub global_status: GlobalStatus,

	/// List of players who have contributed to the creation of this map.
	#[serde(deserialize_with = "crate::serde::vec::deserialize_non_empty")]
	pub mappers: Vec<SteamID>,

	/// List of courses on this map.
	#[serde(deserialize_with = "NewMap::deserialize_courses")]
	pub courses: Vec<NewCourse>,
}

impl NewMap {
	/// Deserializes and validates submitted courses.
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<Vec<NewCourse>, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de;

		use crate::serde::vec;

		let courses: Vec<NewCourse> = vec::deserialize_non_empty(deserializer)?;
		let mut names = HashSet::new();

		for name in courses.iter().filter_map(|course| course.name.as_deref()) {
			if !names.insert(name) {
				return Err(de::Error::custom(format_args!(
					"cannot submit duplicate course `{name}`",
				)));
			}
		}

		Ok(courses)
	}
}

/// Request body for submitting new map courses.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewCourse {
	/// The course's name.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub name: Option<String>,

	/// The course's description.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// List of players who have contributed to the creation of this course.
	#[serde(deserialize_with = "crate::serde::vec::deserialize_non_empty")]
	pub mappers: Vec<SteamID>,

	/// The course's filters.
	#[serde(deserialize_with = "NewCourse::deserialize_filters")]
	pub filters: [NewFilter; 4],
}

impl NewCourse {
	/// Deserializes and validates submitted course filters.
	///
	/// This will enforce the following invariants:
	///   - There are exactly 4 filters
	///   - [`NewCourse::filters`] is sorted
	///   - All 4 permutations of (mode, teleports) are covered
	///   - Any filters with a tier higher than [`Tier::Death`] cannot also be marked as
	///     [`RankedStatus::Ranked`]
	fn deserialize_filters<'de, D>(deserializer: D) -> Result<[NewFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de;

		let mut filters = <[NewFilter; 4]>::deserialize(deserializer)?;

		filters.sort_by_key(|filter| (filter.mode, filter.teleports));

		/// The expected set of filters.
		const EXPECTED: [(Mode, bool); 4] = [
			(Mode::Vanilla, false),
			(Mode::Vanilla, true),
			(Mode::Classic, false),
			(Mode::Classic, true),
		];

		for (filter, expected) in iter::zip(&filters, EXPECTED) {
			if (filter.mode, filter.teleports) != expected {
				return Err(de::Error::custom(format_args!(
					"filter for ({}, {}) is missing",
					filter.mode,
					if filter.teleports { "TP" } else { "Pro" },
				)));
			}

			if filter.tier > Tier::Death && filter.ranked_status == RankedStatus::Ranked {
				return Err(de::Error::custom(format_args!(
					"tier `{}` is too high for a ranked filter",
					filter.tier,
				)));
			}
		}

		Ok(filters)
	}
}

/// Request body for submitting new course filters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewFilter {
	/// The mode this filter applies to.
	pub mode: Mode,

	/// The "runtype" this filter applies to (whether teleports are used or not).
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Extra notes about this filter.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub notes: Option<String>,
}

/// A newly created map.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedMap {
	/// The map's ID.
	#[schema(value_type = u16)]
	pub map_id: NonZeroU16,
}

/// Request body for updating maps.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MapUpdate {
	/// A new description.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// A new workshop ID.
	///
	/// Setting this parameter implies setting `check_steam=true` and has precedence over it.
	pub workshop_id: Option<u32>,

	/// A new global status.
	pub global_status: Option<GlobalStatus>,

	/// Whether to check steam for an updated name and checksum.
	#[serde(default)]
	pub check_steam: bool,

	/// Players to be added as mappers of this map.
	#[serde(default, deserialize_with = "crate::serde::vec::deserialize_empty_as_none")]
	pub added_mappers: Option<Vec<SteamID>>,

	/// Players to be removed as mappers of this map.
	#[serde(default, deserialize_with = "crate::serde::vec::deserialize_empty_as_none")]
	pub removed_mappers: Option<Vec<SteamID>>,

	/// Updates to courses on this map.
	///
	/// course ID -> update payload
	#[serde(default, deserialize_with = "crate::serde::btree_map::deserialize_empty_as_none")]
	#[schema(example = json!({
	  "1": {
	    "name": "foobar"
	  },
	  "2": {
	    "description": "cool course!"
	  }
	}))]
	pub course_updates: Option<BTreeMap<NonZeroU32, CourseUpdate>>,
}

/// Request body for updating courses.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct CourseUpdate {
	/// A new name.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub name: Option<String>,

	/// A new description.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// Players to be added as mappers of this course.
	#[serde(default, deserialize_with = "crate::serde::vec::deserialize_empty_as_none")]
	pub added_mappers: Option<Vec<SteamID>>,

	/// Players to be removed as mappers of this course.
	#[serde(default, deserialize_with = "crate::serde::vec::deserialize_empty_as_none")]
	pub removed_mappers: Option<Vec<SteamID>>,

	/// Updates to any filters of this course.
	#[serde(default, deserialize_with = "crate::serde::btree_map::deserialize_empty_as_none")]
	#[schema(example = json!({
	  "1": {
	    "name": "foobar"
	  },
	  "2": {
	    "description": "cool course!"
	  }
	}))]
	pub filter_updates: Option<BTreeMap<NonZeroU32, FilterUpdate>>,
}

/// Request body for updating course filters.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct FilterUpdate {
	/// A new tier.
	pub tier: Option<Tier>,

	/// A new ranked status.
	pub ranked_status: Option<RankedStatus>,

	/// New notes.
	#[serde(default, deserialize_with = "crate::serde::string::deserialize_empty_as_none")]
	pub notes: Option<String>,
}

/// Information about a map.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct MapInfo {
	/// The map's ID.
	#[sqlx(rename = "map_id", try_from = "u16")]
	#[schema(value_type = u16)]
	pub id: NonZeroU16,

	/// The map's name.
	#[sqlx(rename = "map_name")]
	pub name: String,
}

/// Information about a course.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct CourseInfo {
	/// The course's ID.
	#[sqlx(rename = "course_id", try_from = "u32")]
	#[schema(value_type = u32)]
	pub id: NonZeroU32,

	/// The course's name.
	#[sqlx(rename = "course_name")]
	pub name: Option<String>,

	/// The course filter's tier.
	#[sqlx(rename = "course_tier")]
	pub tier: Tier,
}
