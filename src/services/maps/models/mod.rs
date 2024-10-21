//! Request / Response types for this service.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::{cmp, iter};

use axum::response::{AppendHeaders, IntoResponse, Response};
use cs2kz::{GlobalStatus, Mode, RankedStatus, SteamID, Tier};
use serde::{Deserialize, Deserializer, Serialize};
use tap::{Conv, Tap};
use time::OffsetDateTime;

use crate::num::ClampedU64;
use crate::services::players::PlayerInfo;
use crate::services::steam::WorkshopID;
use crate::util::MapIdentifier;

#[doc(hidden)]
pub(crate) mod checksum;
pub use checksum::Checksum;

crate::macros::make_id! {
	/// A unique identifier for a KZ map.
	MapID as u16
}

crate::macros::make_id! {
	/// A unique identifier for a KZ map course.
	CourseID as u16
}

crate::macros::make_id! {
	/// A unique identifier for a KZ map course filter.
	FilterID as u16
}

/// Request payload for fetching a map.
#[derive(Debug)]
pub struct FetchMapRequest
{
	/// An identifier specifying which map you want to fetch.
	pub ident: MapIdentifier,
}

/// Response payload for fetching a map.
#[derive(Debug, PartialEq, Serialize, utoipa::ToSchema, utoipa::IntoResponses)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[response(status = OK)]
pub struct FetchMapResponse
{
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

	/// Checksum of the map's `.vpk` file.
	pub checksum: Checksum,

	/// Players who contributed to the creation of this map.
	pub mappers: Vec<PlayerInfo>,

	/// The map's courses.
	pub courses: Vec<Course>,

	/// When this map was approved.
	#[serde(with = "time::serde::rfc3339")]
	pub created_on: OffsetDateTime,
}

// We can't derive this because of how we use `Vec` here. We aren't _actually_
// decoding arrays here, but just one element and then put that in a `Vec`.
// MySQL doesn't support arrays anyway, so we have to tell sqlx how to decode
// this type manually.
impl<'r, R> sqlx::FromRow<'r, R> for FetchMapResponse
where
	R: sqlx::Row,
	for<'a> &'a str: sqlx::ColumnIndex<R>,
	MapID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	String: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	GlobalStatus: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	WorkshopID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	Checksum: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	SteamID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	Course: sqlx::FromRow<'r, R>,
	OffsetDateTime: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
{
	fn from_row(row: &'r R) -> sqlx::Result<Self>
	{
		let id = row.try_get("id")?;
		let name = row.try_get("name")?;
		let description = row.try_get("description")?;
		let global_status = row.try_get("global_status")?;
		let workshop_id = row.try_get("workshop_id")?;
		let checksum = row.try_get("checksum")?;
		let mappers = vec![PlayerInfo {
			name: row.try_get("mapper_name")?,
			steam_id: row.try_get("mapper_id")?,
		}];
		let courses = vec![Course::from_row(row)?];
		let created_on = row.try_get("created_on")?;

		Ok(Self {
			id,
			name,
			description,
			global_status,
			workshop_id,
			checksum,
			mappers,
			courses,
			created_on,
		})
	}
}

impl IntoResponse for FetchMapResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// A KZ map course.
#[derive(Debug, PartialEq, Serialize, utoipa::ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Course
{
	/// The course's ID.
	pub id: CourseID,

	/// The course's name.
	pub name: String,

	/// Description of the course.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// Players who contributed to the creation of this course.
	pub mappers: Vec<PlayerInfo>,

	/// The course's filters.
	pub filters: Vec<Filter>,
}

// We can't derive thish because of how we use `Vec` here.
// See `FetchMapResponse`'s impl for more details.
impl<'r, R> sqlx::FromRow<'r, R> for Course
where
	R: sqlx::Row,
	for<'a> &'a str: sqlx::ColumnIndex<R>,
	CourseID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	String: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	SteamID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	Filter: sqlx::FromRow<'r, R>,
{
	fn from_row(row: &'r R) -> sqlx::Result<Self>
	{
		let id = row.try_get("course_id")?;
		let name = row.try_get("course_name")?;
		let description = row.try_get("course_description")?;
		let mappers = vec![PlayerInfo {
			name: row.try_get("course_mapper_name")?,
			steam_id: row.try_get("course_mapper_id")?,
		}];
		let filters = vec![Filter::from_row(row)?];

		Ok(Self { id, name, description, mappers, filters })
	}
}

/// A KZ map course filter.
#[derive(Debug, PartialEq, Serialize, sqlx::FromRow, utoipa::ToSchema)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Filter
{
	/// The filter's ID.
	#[sqlx(rename = "filter_id")]
	pub id: FilterID,

	/// The mode associated with this filter.
	#[sqlx(rename = "filter_mode")]
	pub mode: Mode,

	/// Whether this filter is for teleport runs.
	#[sqlx(rename = "filter_teleports")]
	pub teleports: bool,

	/// The filter's tier.
	#[sqlx(rename = "filter_tier")]
	pub tier: Tier,

	/// The filter's ranked status.
	#[sqlx(rename = "filter_ranked_status")]
	pub ranked_status: RankedStatus,

	/// Any additional notes.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[sqlx(rename = "filter_notes")]
	pub notes: Option<String>,
}

/// Request payload for fetching maps.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct FetchMapsRequest
{
	/// Filter by name.
	pub name: Option<String>,

	/// Filter by workshop ID.
	pub workshop_id: Option<WorkshopID>,

	/// Filter by global status.
	pub global_status: Option<GlobalStatus>,

	/// Only include maps approved after this date.
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub created_after: Option<OffsetDateTime>,

	/// Only include maps approved before this date.
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub created_before: Option<OffsetDateTime>,

	/// Maximum number of results to return.
	#[serde(default)]
	#[param(value_type = u64)]
	pub limit: ClampedU64<{ u64::MAX }>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching maps.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[response(status = OK)]
pub struct FetchMapsResponse
{
	/// The maps.
	pub maps: Vec<FetchMapResponse>,

	/// How many maps **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchMapsResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for submitting a new map.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(example = json!({
  "workshop_id": 3070194623_u32,
  "global_status": "global",
  "mappers": ["76561198165203332"],
  "courses": [
    {
      "name": "Main",
      "description": "the main course!",
      "mappers": ["76561198165203332"],
      "filters": [
        {
          "mode": "vanilla",
          "teleports": true,
          "tier": "hard",
          "ranked_status": "ranked",
          "notes": "gotta hit the funny jump :tf:"
        },
        {
          "mode": "vanilla",
          "teleports": false,
          "tier": "very_hard",
          "ranked_status": "ranked"
        },
        {
          "mode": "classic",
          "teleports": true,
          "tier": "easy",
          "ranked_status": "ranked"
        },
        {
          "mode": "classic",
          "teleports": false,
          "tier": "medium",
          "ranked_status": "ranked"
        }
      ]
    }
  ]
}))]
pub struct SubmitMapRequest
{
	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// Description of the map.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// The map's global status.
	pub global_status: GlobalStatus,

	/// List of SteamIDs of the players who contributed to the creation of this
	/// map.
	#[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
	pub mappers: BTreeSet<SteamID>,

	/// The map's courses.
	#[serde(deserialize_with = "SubmitMapRequest::deserialize_courses")]
	pub courses: Vec<NewCourse>,
}

impl SubmitMapRequest
{
	/// Deserializes [`SubmitMapRequest::courses`] and performs validations.
	///
	/// Currently this only checks for duplicate course names.
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<Vec<NewCourse>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let courses = crate::serde::deserialize_non_empty::<Vec<NewCourse>, _>(deserializer)?;
		let mut names = HashSet::new();

		if let Some(name) = courses
			.iter()
			.filter_map(|c| c.name.as_deref())
			.find(|&name| !names.insert(name))
		{
			return Err(serde::de::Error::custom(format!(
				"cannot submit duplicate course `{name}`",
			)));
		}

		Ok(courses)
	}
}

/// Request payload for a course when submitting a new map.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(example = json!({
  "name": "Main",
  "description": "the main course!",
  "mappers": ["76561198165203332"],
  "filters": [
    {
      "mode": "vanilla",
      "teleports": true,
      "tier": "hard",
      "ranked_status": "ranked",
      "notes": "gotta hit the funny jump :tf:"
    },
    {
      "mode": "vanilla",
      "teleports": false,
      "tier": "very_hard",
      "ranked_status": "ranked"
    },
    {
      "mode": "classic",
      "teleports": true,
      "tier": "easy",
      "ranked_status": "ranked"
    },
    {
      "mode": "classic",
      "teleports": false,
      "tier": "medium",
      "ranked_status": "ranked"
    }
  ]
}))]
pub struct NewCourse
{
	/// The course's name.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub name: Option<String>,

	/// Description of the course.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// List of SteamIDs of the players who contributed to the creation of this
	/// course.
	#[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
	pub mappers: BTreeSet<SteamID>,

	/// The course's filters.
	#[serde(deserialize_with = "NewCourse::deserialize_filters")]
	pub filters: [NewFilter; 4],
}

impl NewCourse
{
	/// Deserializes [`NewCourse::filters`] and performs validations.
	///
	/// Currently this makes sure that:
	/// - the 4 filters are actually the 4 possible permutations
	/// - no T9+ filter is marked as "ranked"
	fn deserialize_filters<'de, D>(deserializer: D) -> Result<[NewFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		/// All the permutations of (mode, runtype) that we expect in a filter.
		const ALL_FILTERS: [(Mode, bool); 4] = [
			(Mode::Vanilla, false),
			(Mode::Vanilla, true),
			(Mode::Classic, false),
			(Mode::Classic, true),
		];

		let filters = <[NewFilter; 4]>::deserialize(deserializer)?.tap_mut(|filters| {
			filters.sort_unstable_by_key(|filter| (filter.mode, filter.teleports));
		});

		for (actual, expected) in iter::zip(&filters, ALL_FILTERS) {
			if (actual.mode, actual.teleports) != expected {
				return Err(serde::de::Error::custom(format!(
					"filter for {} {} is missing",
					expected.0.as_str_short(),
					if expected.1 { "Standard" } else { "Pro" },
				)));
			}

			if actual.tier > Tier::Death && actual.ranked_status.is_ranked() {
				return Err(serde::de::Error::custom(format!(
					"tier {} is too high for a ranked filter",
					actual.tier.conv::<u8>(),
				)));
			}
		}

		Ok(filters)
	}
}

/// Request payload for a course filter when submitting a new map.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(example = json!({
  "mode": "vanilla",
  "teleports": true,
  "tier": "hard",
  "ranked_status": "ranked",
  "notes": "gotta hit the funny jump :tf:"
}))]
pub struct NewFilter
{
	/// The mode associated with this filter.
	pub mode: Mode,

	/// Whether this filter is for teleport runs.
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Any additional notes.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub notes: Option<String>,
}

/// Response payload for submitting a new map.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct SubmitMapResponse
{
	/// The map's ID.
	pub map_id: MapID,

	/// IDs related to the created courses.
	pub courses: Vec<CreatedCourse>,
}

impl IntoResponse for SubmitMapResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/maps/{}", self.map_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}

/// Response payload for created courses when submitting a new map.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreatedCourse
{
	/// The course's ID.
	pub id: CourseID,

	/// The IDS of the course's filters.
	pub filter_ids: [FilterID; 4],
}

/// Request payload for updating an existing map.
#[derive(Debug)]
pub struct UpdateMapRequest
{
	/// The ID of the map to update.
	pub map_id: MapID,

	/// A new description.
	pub description: Option<String>,

	/// A new Workshop ID.
	pub workshop_id: Option<WorkshopID>,

	/// A new global status.
	pub global_status: Option<GlobalStatus>,

	/// Whether to check the Workshop for a new name / checksum.
	pub check_steam: bool,

	/// List of SteamIDs of players to add as mappers to this map.
	pub added_mappers: Option<BTreeSet<SteamID>>,

	/// List of SteamIDs of players to remove as mappers from this map.
	pub removed_mappers: Option<BTreeSet<SteamID>>,

	/// Updates to this map's courses.
	pub course_updates: Option<BTreeMap<CourseID, CourseUpdate>>,
}

impl UpdateMapRequest
{
	/// Checks if this update is empty (contains no changes).
	pub fn is_empty(&self) -> bool
	{
		let Self {
			map_id: _,
			description,
			workshop_id,
			global_status,
			check_steam,
			added_mappers,
			removed_mappers,
			course_updates,
		} = self;

		description.is_none()
			&& workshop_id.is_none()
			&& global_status.is_none()
			&& !check_steam
			&& added_mappers.is_none()
			&& removed_mappers.is_none()
			&& course_updates.is_none()
	}
}

/// Response payload for updating an existing map.
#[derive(Debug, Default, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct UpdateMapResponse
{
	/// A list of courses that were updated.
	pub updated_courses: Vec<UpdatedCourse>,
}

impl IntoResponse for UpdateMapResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::OK;
		let body = crate::http::extract::Json(self);

		(status, body).into_response()
	}
}

/// A course that was updated as a result of a map update.
#[derive(Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct UpdatedCourse
{
	/// The course's ID.
	pub id: CourseID,

	/// A list of filter IDs of the filters that were updated as part of this
	/// course update.
	pub updated_filter_ids: Vec<FilterID>,
}

impl PartialOrd for UpdatedCourse
{
	fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering>
	{
		Some(self.cmp(other))
	}
}

impl Ord for UpdatedCourse
{
	fn cmp(&self, other: &Self) -> cmp::Ordering
	{
		self.id.cmp(&other.id)
	}
}

/// An update to a map course.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct CourseUpdate
{
	/// A new name.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub name: Option<String>,

	/// A new description.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// List of SteamIDs of players to add as mappers to this course.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub added_mappers: Option<BTreeSet<SteamID>>,

	/// List of SteamIDs of players to remove as mappers from this course.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub removed_mappers: Option<BTreeSet<SteamID>>,

	/// Updates to this course's filters.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub filter_updates: Option<BTreeMap<FilterID, FilterUpdate>>,
}

impl CourseUpdate
{
	/// Checks if this update is empty (contains no changes).
	pub fn is_empty(&self) -> bool
	{
		let Self { name, description, added_mappers, removed_mappers, filter_updates } = self;

		name.is_none()
			&& description.is_none()
			&& added_mappers.is_none()
			&& removed_mappers.is_none()
			&& filter_updates
				.as_ref()
				.map_or(true, |updates| updates.values().all(FilterUpdate::is_empty))
	}
}

/// An update to a course filter.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct FilterUpdate
{
	/// A new tier.
	pub tier: Option<Tier>,

	/// A new ranked status.
	pub ranked_status: Option<RankedStatus>,

	/// New notes.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
	pub notes: Option<String>,
}

impl FilterUpdate
{
	/// Checks if this update is empty (contains no changes).
	pub fn is_empty(&self) -> bool
	{
		let Self { tier, ranked_status, notes } = self;

		tier.is_none() && ranked_status.is_none() && notes.is_none()
	}
}
