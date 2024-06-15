//! Types for modeling KZ maps.

use std::collections::{BTreeMap, HashSet};
use std::iter;

use chrono::{DateTime, Utc};
use cs2kz::{GlobalStatus, Mode, RankedStatus, SteamID, Tier};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::make_id;
use crate::players::Player;
use crate::steam::workshop::WorkshopID;

make_id!(MapID as u16);
make_id!(CourseID as u16);
make_id!(FilterID as u16);

/// A KZ map.
#[derive(Debug, Serialize, ToSchema)]
pub struct FullMap {
	/// The map's ID.
	pub id: MapID,

	/// The map's name.
	pub name: String,

	/// Description of the map.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// The map's global status.
	pub global_status: GlobalStatus,

	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// CRC32 checksum of the map's `.vpk` file.
	pub checksum: u32,

	/// Players who contributed to the creation of this map.
	pub mappers: Vec<Player>,

	/// The map's courses.
	pub courses: Vec<Course>,

	/// When this map was approved.
	pub created_on: DateTime<Utc>,
}

impl FullMap {
	/// Combines two maps by merging mappers and course information from `other` into `self`.
	///
	/// This function is used for aggregating database results; see `FullMap`'s [`FromRow`]
	/// implementation for more details.
	pub fn reduce(mut self, other: Self) -> Self {
		assert_eq!(self.id, other.id, "merging two unrelated maps");

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

	/// Flatten database results by aggregating maps with equal IDs but different
	/// mappers/courses into a list of maps with unique IDs.
	pub fn flatten<I>(maps: I, limit: usize) -> Vec<Self>
	where
		I: IntoIterator<Item = Self>,
	{
		maps.into_iter()
			.chunk_by(|map| map.id)
			.into_iter()
			.filter_map(|(_, maps)| maps.reduce(Self::reduce))
			.take(limit)
			.collect()
	}
}

impl FromRow<'_, MySqlRow> for FullMap {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
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

/// A KZ map course.
#[derive(Debug, Serialize, ToSchema)]
pub struct Course {
	/// The course's ID.
	pub id: CourseID,

	/// The course's name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// Description of the course.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// Players who contributed to the creation of this course.
	pub mappers: Vec<Player>,

	/// The course's filters.
	pub filters: Vec<Filter>,
}

impl FromRow<'_, MySqlRow> for Course {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("course_id")?,
			name: row.try_get("course_name")?,
			description: row.try_get("course_description")?,
			mappers: vec![Player {
				name: row.try_get("course_mapper_name")?,
				steam_id: row.try_get("course_mapper_id")?,
			}],
			filters: vec![Filter {
				id: row.try_get("filter_id")?,
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
	pub id: FilterID,

	/// The mode associated with this filter.
	pub mode: Mode,

	/// Whether this filter is for teleport runs.
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Any additional notes.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub notes: Option<String>,
}

/// Request payload for creating a new map.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewMap {
	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// Description of the map.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub description: Option<String>,

	/// The map's global status.
	pub global_status: GlobalStatus,

	/// List of SteamIDs of the players who contributed to the creation of this map.
	#[serde(deserialize_with = "crate::serde::vec::deserialize_non_empty")]
	pub mappers: Vec<SteamID>,

	/// The map's courses.
	#[serde(deserialize_with = "NewMap::deserialize_courses")]
	pub courses: Vec<NewCourse>,
}

impl NewMap {
	/// Deserializes courses and (partially) validates them.
	///
	/// This function ensures **logical invariants**, such as:
	///    1. there are no duplicate course names
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<Vec<NewCourse>, D::Error>
	where
		D: Deserializer<'de>,
	{
		use crate::serde::vec;

		let courses: Vec<NewCourse> = vec::deserialize_non_empty(deserializer)?;
		let mut names = HashSet::new();

		if let Some(name) = courses
			.iter()
			.filter_map(|course| course.name.as_deref())
			.find(|&name| !names.insert(name))
		{
			return Err(serde::de::Error::custom(format_args!(
				"cannot submit duplicate course `{name}`",
			)));
		}

		Ok(courses)
	}
}

/// Request payload for creating a new course.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewCourse {
	/// The course's name.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub name: Option<String>,

	/// Description of the course.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub description: Option<String>,

	/// List of SteamIDs of the players who contributed to the creation of this course.
	#[serde(deserialize_with = "crate::serde::vec::deserialize_non_empty")]
	pub mappers: Vec<SteamID>,

	/// The course's filters.
	#[serde(deserialize_with = "NewCourse::deserialize_filters")]
	pub filters: [NewFilter; 4],
}

impl NewCourse {
	/// Deserializes filters and (partially) validates them.
	///
	/// This function ensures **logical invariants**, such as:
	///    1. each of the 4 filters covers one of the 4 (mode, runtype) combinations
	///    2. no filter has a tier above 8 and is marked as "ranked"
	fn deserialize_filters<'de, D>(deserializer: D) -> Result<[NewFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut filters = <[NewFilter; 4]>::deserialize(deserializer)?;

		filters.sort_unstable_by_key(|filter| (filter.mode, filter.teleports));

		#[allow(clippy::missing_docs_in_private_items)]
		const EXPECTED: [(Mode, bool); 4] = [
			(Mode::Vanilla, false),
			(Mode::Vanilla, true),
			(Mode::Classic, false),
			(Mode::Classic, true),
		];

		for (filter, expected) in iter::zip(&filters, EXPECTED) {
			if (filter.mode, filter.teleports) != expected {
				return Err(serde::de::Error::custom(format_args!(
					"filter for ({}, {}) is missing",
					filter.mode,
					if filter.teleports { "TP" } else { "Pro" },
				)));
			}

			if filter.tier > Tier::Death && filter.ranked_status.is_ranked() {
				return Err(serde::de::Error::custom(format_args!(
					"tier `{}` is too high for a ranked filter",
					filter.tier,
				)));
			}
		}

		Ok(filters)
	}
}

/// Request payload for creating a new course filter.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewFilter {
	/// The mode associated with this filter.
	pub mode: Mode,

	/// Whether this filter is for teleport runs.
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Any additional notes.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub notes: Option<String>,
}

/// Response body for creating a new map.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedMap {
	/// The map's ID.
	pub map_id: MapID,
}

/// Request payload for updating an existing map.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MapUpdate {
	/// A new description.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub description: Option<String>,

	/// A new Workshop ID.
	pub workshop_id: Option<WorkshopID>,

	/// A new global status.
	pub global_status: Option<GlobalStatus>,

	/// Whether to check the Workshop for a new name / checksum.
	#[serde(default)]
	pub check_steam: bool,

	/// List of SteamIDs of players to add as mappers to this map.
	#[serde(
		default,
		deserialize_with = "crate::serde::vec::deserialize_empty_as_none"
	)]
	pub added_mappers: Option<Vec<SteamID>>,

	/// List of SteamIDs of players to remove as mappers from this map.
	#[serde(
		default,
		deserialize_with = "crate::serde::vec::deserialize_empty_as_none"
	)]
	pub removed_mappers: Option<Vec<SteamID>>,

	/// Updates to this map's courses.
	#[serde(
		default,
		deserialize_with = "crate::serde::btree_map::deserialize_empty_as_none"
	)]
	#[schema(example = json!({
	  "1": {
	    "name": "foobar"
	  },
	  "2": {
	    "description": "cool course!"
	  }
	}))]
	pub course_updates: Option<BTreeMap<CourseID, CourseUpdate>>,
}

/// Request payload for updating a map course.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct CourseUpdate {
	/// A new name.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub name: Option<String>,

	/// A new description.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub description: Option<String>,

	/// List of SteamIDs of players to add as mappers to this course.
	#[serde(
		default,
		deserialize_with = "crate::serde::vec::deserialize_empty_as_none"
	)]
	pub added_mappers: Option<Vec<SteamID>>,

	/// List of SteamIDs of players to remove as mappers from this course.
	#[serde(
		default,
		deserialize_with = "crate::serde::vec::deserialize_empty_as_none"
	)]
	pub removed_mappers: Option<Vec<SteamID>>,

	/// Updates to this course's filters.
	#[serde(
		default,
		deserialize_with = "crate::serde::btree_map::deserialize_empty_as_none"
	)]
	#[schema(example = json!({
	  "1": {
	    "name": "foobar"
	  },
	  "2": {
	    "description": "cool course!"
	  }
	}))]
	pub filter_updates: Option<BTreeMap<FilterID, FilterUpdate>>,
}

/// Request payload for updating a course filter.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct FilterUpdate {
	/// A new tier.
	pub tier: Option<Tier>,

	/// A new ranked status.
	pub ranked_status: Option<RankedStatus>,

	/// New notes.
	#[serde(
		default,
		deserialize_with = "crate::serde::string::deserialize_empty_as_none"
	)]
	pub notes: Option<String>,
}

/// Information about a KZ map.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct MapInfo {
	/// The map's ID.
	#[sqlx(rename = "map_id")]
	pub id: MapID,

	/// The map's name.
	#[sqlx(rename = "map_name")]
	pub name: String,
}

/// Information about a KZ map course.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct CourseInfo {
	/// The course's ID.
	#[sqlx(rename = "course_id")]
	pub id: CourseID,

	/// The course's name.
	#[sqlx(rename = "course_name")]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// The course's tier.
	#[sqlx(rename = "course_tier")]
	pub tier: Tier,
}
