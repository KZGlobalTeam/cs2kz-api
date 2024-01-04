//! This module holds types related to KZ players.

use std::fmt::Display;
use std::result::Result as StdResult;

use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Style, Tier};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::mysql::{MySqlQueryResult, MySqlRow};
use sqlx::{FromRow, MySqlExecutor, QueryBuilder, Row};
use utoipa::ToSchema;

use super::Player;
use crate::{Error, Result};

/// Information about a map.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
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
          "mode": "kz_classic",
          "teleports": true,
          "tier": 3,
          "ranked_status": "ranked"
        },
        {
          "mode": "kz_classic",
          "teleports": false,
          "tier": 4,
          "ranked_status": "ranked"
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
	pub mappers: Vec<Player>,

	/// List of courses on this map.
	pub courses: Vec<Course>,

	/// The filesize of this map in bytes.
	pub filesize: u64,

	/// Timestamp of when this map was initially approved.
	pub created_on: DateTime<Utc>,

	/// Timestamp of when this map was last updated.
	pub updated_on: DateTime<Utc>,
}

impl KZMap {
	/// Fetches courses for the map with the given `map_id`.
	pub async fn get_courses(
		map_id: u16,
		connection: impl MySqlExecutor<'_>,
	) -> Result<Vec<CourseRow>> {
		sqlx::query_as! {
			CourseRow,
			r#"
			SELECT
				id,
				map_stage
			FROM
				Courses
			WHERE
				map_id = ?
			"#,
			map_id,
		}
		.fetch_all(connection)
		.await
		.map_err(Into::into)
	}

	/// Creates a batch of new `mappers` for the map with the given `map_id`.
	pub async fn create_mappers(
		map_id: u16,
		mappers: &[SteamID],
		connection: impl MySqlExecutor<'_>,
	) -> sqlx::Result<MySqlQueryResult> {
		let mut query = QueryBuilder::new("INSERT INTO Mappers (map_id, player_id)");

		query.push_values(mappers, |mut query, mapper| {
			query.push_bind(map_id).push_bind(mapper);
		});

		query.build().execute(connection).await
	}

	/// Deletes all the given `mappers` for the map with ID `map_id`.
	pub async fn delete_mappers(
		map_id: u16,
		mappers: &[SteamID],
		connection: impl MySqlExecutor<'_>,
	) -> sqlx::Result<MySqlQueryResult> {
		let mut query = QueryBuilder::new("DELETE FROM Mappers WHERE (map_id, player_id) IN");

		query.push_tuples(mappers, |mut query, steam_id| {
			query.push_bind(map_id).push_bind(steam_id);
		});

		query.build().execute(connection).await
	}

	/// Creates courses for all given `stages` for the map with ID `map_id`.
	pub async fn create_courses(
		map_id: u16,
		stages: impl Iterator<Item = u8>,
		connection: impl MySqlExecutor<'_>,
	) -> sqlx::Result<MySqlQueryResult> {
		let mut query = QueryBuilder::new("INSERT INTO Courses (map_id, map_stage)");

		query.push_values(stages, |mut query, stage| {
			query.push_bind(map_id).push_bind(stage);
		});

		query.build().execute(connection).await
	}

	/// Deletes all courses associated with the given `course_ids`.
	pub async fn delete_courses(
		course_ids: &[u32],
		connection: impl MySqlExecutor<'_>,
	) -> sqlx::Result<MySqlQueryResult> {
		let mut query = QueryBuilder::new("DELETE FROM Courses WHERE (id) IN");

		query.push_tuples(course_ids, |mut query, course_id| {
			query.push_bind(course_id);
		});

		query.build().execute(connection).await
	}

	/// [`Iterator::reduce`] function for folding multiple [`KZMap`]s into one.
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

	/// Flattens a list of maps with different mappers and courses but otherwise duplicate
	/// data.
	pub fn flatten(maps: Vec<Self>) -> Vec<Self> {
		maps.into_iter()
			.group_by(|map| map.id)
			.into_iter()
			.filter_map(|(_, maps)| maps.reduce(Self::reduce))
			.collect()
	}
}

/// Information about a course.
#[derive(Debug, Serialize, ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
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
      "mode": "kz_classic",
      "teleports": true,
      "tier": 3,
      "ranked_status": "ranked"
    },
    {
      "mode": "kz_classic",
      "teleports": false,
      "tier": 4,
      "ranked_status": "ranked"
    }
  ]
}))]
pub struct Course {
	/// The course's ID.
	pub id: u32,

	/// The stage of the map this course corresponds to.
	pub stage: u8,

	/// List of the players who created this course.
	pub mappers: Vec<Player>,

	/// List of filters that apply to this course.
	pub filters: Vec<Filter>,
}

impl Course {
	/// Creates mappers for the given `courses` according to the given `params`.
	pub async fn create_mappers(
		courses: &[CourseRow],
		params: &[CreateCourseParams],
		connection: impl MySqlExecutor<'_>,
	) -> Result<MySqlQueryResult> {
		let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

		let iter = params
			.iter()
			.map(|course| {
				let course_id = CourseRow::find_by_stage(course.stage, courses.iter().copied())?;

				Ok(course
					.mappers
					.iter()
					.map(move |steam_id| (course_id, steam_id)))
			})
			.collect::<Result<Vec<_>>>()? // Maybe we can do better here?
			.into_iter()
			.flatten();

		query.push_values(iter, |mut query, (course_id, mapper)| {
			query.push_bind(course_id).push_bind(mapper);
		});

		query.build().execute(connection).await.map_err(Into::into)
	}

	/// Creates filters for the given `courses` according to the given `params`.
	pub async fn create_filters(
		courses: &[CourseRow],
		params: &[CreateCourseParams],
		connection: impl MySqlExecutor<'_>,
	) -> Result<MySqlQueryResult> {
		let mut query = QueryBuilder::new(
			r#"
			INSERT INTO
				CourseFilters (course_id, mode_id, teleports, tier, ranked)
			"#,
		);

		let iter = params
			.iter()
			.map(|course| {
				let course_id = CourseRow::find_by_stage(course.stage,courses.iter().copied())?;

				Ok(course
					.filters
					.iter()
					.map(move |filter| (course_id, filter)))
			})
			.collect::<Result<Vec<_>>>()? // Maybe we can do better here?
			.into_iter()
			.flatten();

		query.push_values(iter, |mut query, (course_id, filter)| {
			query
				.push_bind(course_id)
				.push_bind(filter.mode)
				.push_bind(filter.teleports)
				.push_bind(filter.tier)
				.push_bind(filter.ranked_status);
		});

		query.build().execute(connection).await.map_err(Into::into)
	}
}

/// A database row representing a course.
#[derive(Debug, Clone, Copy, FromRow)]
pub struct CourseRow {
	id: u32,
	map_stage: u8,
}

impl CourseRow {
	fn find_by_stage(stage: u8, mut courses: impl Iterator<Item = Self>) -> Result<u32> {
		courses
			.find(|course| course.map_stage == stage)
			.map(|course| course.id)
			.ok_or(Error::MissingCourse { stage })
	}
}

/// A new course.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "stage": 0,
  "mappers": ["STEAM_1:0:102468802"],
  "filters": [
    {
      "mode": "kz_classic",
      "teleports": true,
      "tier": 3,
      "ranked_status": "ranked"
    },
    {
      "mode": "kz_classic",
      "teleports": false,
      "tier": 4,
      "ranked_status": "ranked"
    }
  ]
}))]
pub struct CreateCourseParams {
	/// The stage of the map this course corresponds to.
	pub stage: u8,

	/// List of the players who created this course.
	pub mappers: Vec<SteamID>,

	/// List of filters that apply to this course.
	pub filters: Vec<Filter>,
}

/// Information about a course filter.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
  "id": 1,
  "mode": "kz_classic",
  "teleports": true,
  "tier": 3,
  "ranked_status": "ranked",
}))]
pub struct Filter {
	/// The filter's ID.
	pub id: u32,

	/// The mode for this filter.
	pub mode: Mode,

	/// Whether this filter applies to runs with teleports.
	pub teleports: bool,

	/// The difficulty of the course with this filter.
	#[serde(serialize_with = "Tier::serialize_integer")]
	#[schema(value_type = u8, minimum = 1, maximum = 10)]
	pub tier: Tier,

	/// The ranked status of this filter.
	pub ranked_status: RankedStatus,
}

/// The ranked status of a [Filter].
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	sqlx::Type,
	ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum RankedStatus {
	/// This filter will never be ranked.
	///
	/// This is the case if either the mapper decided they don't want the filter to
	/// be ranked, or because it doesn't meet the minimum requirements for ranking.
	Never = -1,

	/// This filter is unranked, because it has no completions yet.
	Unranked = 0,

	/// This filter is ranked.
	Ranked = 1,
}

impl Display for RankedStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

impl TryFrom<i8> for RankedStatus {
	type Error = ();

	fn try_from(value: i8) -> StdResult<Self, Self::Error> {
		match value {
			-1 => Ok(Self::Never),
			0 => Ok(Self::Unranked),
			1 => Ok(Self::Ranked),
			_ => Err(()),
		}
	}
}

impl FromRow<'_, MySqlRow> for KZMap {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
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
		let mappers = vec![Player { steam_id: mapper_steam_id, name: mapper_name }];

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

		let filter_id = row.try_get("filter_id")?;
		let filter_teleports = row.try_get("filter_teleports")?;
		let filter_tier = row.try_get("filter_tier")?;
		let filter_ranked = row
			.try_get::<i8, _>("filter_ranked")?
			.try_into()
			.expect("invalid `ranked_status`");

		let courses = vec![Course {
			id: course_id,
			stage: course_stage,
			mappers: vec![Player { steam_id: course_mapper_steam_id, name: course_mapper_name }],
			filters: vec![Filter {
				id: filter_id,
				mode: filter_mode,
				teleports: filter_teleports,
				tier: filter_tier,
				ranked_status: filter_ranked,
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
