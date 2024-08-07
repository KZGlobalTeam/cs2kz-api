//! A service for managing KZ maps.

use std::collections::HashSet;
use std::{fmt, iter};

use axum::extract::FromRef;
use cs2kz::{GlobalStatus, SteamID};
use futures::{TryFutureExt, TryStreamExt};
use itertools::Itertools;
use sqlx::{MySql, Pool, QueryBuilder, Row, Transaction};
use tap::{Pipe, Tap, TryConv};

use crate::database::SqlErrorExt;
use crate::services::steam::WorkshopID;
use crate::services::{AuthService, SteamService};

pub(crate) mod http;
mod queries;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	Checksum,
	Course,
	CourseID,
	CourseUpdate,
	CreatedCourse,
	FetchMapRequest,
	FetchMapResponse,
	FetchMapsRequest,
	FetchMapsResponse,
	Filter,
	FilterID,
	FilterUpdate,
	MapID,
	NewCourse,
	NewFilter,
	SubmitMapRequest,
	SubmitMapResponse,
	UpdateMapRequest,
	UpdateMapResponse,
	UpdatedCourse,
};

/// A service for managing KZ maps.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct MapService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
	steam_svc: SteamService,
}

impl fmt::Debug for MapService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("MapService").finish_non_exhaustive()
	}
}

impl MapService
{
	/// Create a new [`MapService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>, auth_svc: AuthService, steam_svc: SteamService) -> Self
	{
		Self { database, auth_svc, steam_svc }
	}

	/// Fetch a map.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_map(&self, req: FetchMapRequest) -> Result<Option<FetchMapResponse>>
	{
		let raw_maps = sqlx::query_as::<_, FetchMapResponse>(&format!(
			r"
			{}
			WHERE
			  m.id = COALESCE(?, m.id)
			  AND m.name LIKE COALESCE(?, m.name)
			",
			queries::SELECT,
		))
		.bind(req.ident.as_id())
		.bind(req.ident.as_name().map(|name| format!("%{name}%")))
		.fetch_all(&self.database)
		.await?;

		let Some(map_id) = raw_maps.first().map(|m| m.id) else {
			return Ok(None);
		};

		let map = raw_maps
			.into_iter()
			.filter(|m| m.id == map_id)
			.reduce(reduce_chunk)
			.expect("we got the id we're filtering by from the original list");

		Ok(Some(map))
	}

	/// Fetch maps.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_maps(&self, req: FetchMapsRequest) -> Result<FetchMapsResponse>
	{
		let map_chunks = sqlx::query_as::<_, FetchMapResponse>(&format!(
			r"
			{}
			WHERE
			  m.name LIKE COALESCE(?, m.name)
			  AND m.workshop_id = COALESCE(?, m.workshop_id)
			  AND m.global_status = COALESCE(?, m.global_status)
			  AND m.created_on > COALESCE(?, '1970-01-01 00:00:01')
			  AND m.created_on < COALESCE(?, '2038-01-19 03:14:07')
			ORDER BY
			  m.id DESC
			",
			queries::SELECT,
		))
		.bind(req.name.map(|name| format!("%{name}%")))
		.bind(req.workshop_id)
		.bind(req.global_status)
		.bind(req.created_after)
		.bind(req.created_before)
		.fetch_all(&self.database)
		.await?
		.into_iter()
		.chunk_by(|m| m.id);

		// Take into account how many maps we're gonna skip over
		let mut total = *req.offset;

		let maps = map_chunks
			.into_iter()
			.map(|(_, chunk)| chunk.reduce(reduce_chunk).expect("chunk can't be empty"))
			.skip(*req.offset as usize)
			.take(*req.limit as usize)
			.collect_vec();

		// Add all the maps we actually return
		total += maps.len() as u64;

		// And everything else that we would have ignored otherwise
		total += map_chunks.into_iter().count() as u64;

		Ok(FetchMapsResponse { maps, total })
	}

	/// Submit a new map.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn submit_map(&self, req: SubmitMapRequest) -> Result<SubmitMapResponse>
	{
		if req.mappers.is_empty() {
			return Err(Error::MapMustHaveMappers);
		}

		if req.courses.is_empty() {
			return Err(Error::MapMustHaveCourses);
		}

		if req.courses.iter().any(|c| c.mappers.is_empty()) {
			return Err(Error::CourseMustHaveMappers { course_id: None });
		}

		let mut txn = self.database.begin().await?;

		let (map_name, checksum) = tokio::try_join! {
			self.steam_svc.fetch_map_name(req.workshop_id).map_err(Error::Steam),

			// TODO: put this in background task and return `202`?
			self.steam_svc.download_map(req.workshop_id)
				.map_err(Error::Steam)
				.and_then(|map_file| {
					map_file.checksum()
						.map_ok(Checksum::from)
						.map_err(Error::CalculateMapChecksum)
				}),
		}?;

		let map_id = create_map(&map_name, checksum, &req, &mut txn).await?;
		create_mappers(map_id, &req.mappers, &mut txn).await?;
		let courses = create_courses(map_id, &req.courses, &mut txn).await?;

		txn.commit().await?;

		Ok(SubmitMapResponse { map_id, courses })
	}

	/// Update an existing map.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn update_map(&self, req: UpdateMapRequest) -> Result<UpdateMapResponse>
	{
		let mut response = UpdateMapResponse::default();

		if req.is_empty() {
			return Ok(response);
		}

		let mut txn = self.database.begin().await?;

		update_metadata(&req, &mut txn).await?;

		if req.check_steam || req.workshop_id.is_some() {
			check_steam(&req, &mut txn, &self.steam_svc).await?;
		}

		if let Some(mappers) = req.added_mappers {
			create_mappers(req.map_id, &mappers, &mut txn).await?;
		}

		if let Some(mappers) = req.removed_mappers {
			remove_mappers(req.map_id, &mappers, &mut txn).await?;
		}

		if let Some(updates) = req.course_updates {
			response.updated_courses = update_courses(req.map_id, updates, &mut txn).await?;
		}

		txn.commit().await?;

		tracing::info!(target: "cs2kz_api::audit_log", map_id = %req.map_id, "updated map");

		Ok(response)
	}
}

/// Reduce function for merging multiple database results for the same map with
/// different mappers and courses.
///
/// When we fetch maps from the DB, we get "duplicates" for maps with multiple
/// mappers and/or courses, since SQL doesn't support arrays. All the
/// information in these results is the same, except for the mapper/course
/// information. We group results by their ID and then reduce each chunk down
/// into a single map using this function.
fn reduce_chunk(mut acc: FetchMapResponse, curr: FetchMapResponse) -> FetchMapResponse
{
	assert_eq!(acc.id, curr.id, "merging two unrelated maps");

	for mapper in curr.mappers {
		if !acc.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
			acc.mappers.push(mapper);
		}
	}

	for course in curr.courses {
		let Some(c) = acc.courses.iter_mut().find(|c| c.id == course.id) else {
			acc.courses.push(course);
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

	acc
}

/// Creates a new map in the database and returns the generated ID.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn create_map(
	map_name: &str,
	checksum: Checksum,
	req: &SubmitMapRequest,
	txn: &mut Transaction<'_, MySql>,
) -> Result<MapID>
{
	let deglobal_result = sqlx::query! {
		r"
		UPDATE
		  Maps
		SET
		  global_status = ?
		WHERE
		  name = ?
		",
		GlobalStatus::NotGlobal,
		map_name,
	}
	.execute(txn.as_mut())
	.await?;

	match deglobal_result.rows_affected() {
		0 => { /* all good, this is a new map */ }
		1 => tracing::info! {
			target: "cs2kz_api::audit_log",
			%map_name,
			"degloballed old version of map",
		},
		amount => tracing::warn! {
			%map_name,
			%amount,
			"degloballed multiple old versions of map",
		},
	}

	let map_id = sqlx::query! {
		r"
		INSERT INTO
		  Maps (
		    name,
		    description,
		    global_status,
		    workshop_id,
		    checksum
		  )
		VALUES
		  (?, ?, ?, ?, ?)
		RETURNING id
		",
		map_name,
		req.description,
		req.global_status,
		req.workshop_id,
		checksum,
	}
	.fetch_one(txn.as_mut())
	.await
	.and_then(|row| row.try_get(0))?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		id = %map_id,
		name = %map_name,
		new = %(deglobal_result.rows_affected() == 0),
		"created map",
	};

	Ok(map_id)
}

/// Inserts mappers into the database.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn create_mappers(
	map_id: MapID,
	mapper_ids: &[SteamID],
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	QueryBuilder::new(queries::INSERT_MAPPERS)
		.tap_mut(|query| {
			query.push_values(mapper_ids, |mut query, mapper_id| {
				query.push_bind(map_id).push_bind(mapper_id);
			});
		})
		.build()
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("player_id") {
				Error::MapperDoesNotExist
			} else {
				Error::Database(error)
			}
		})?;

	tracing::debug!(target: "cs2kz_api::audit_log", ?mapper_ids, "created mappers");

	Ok(())
}

/// Inserts submitted map courses into the database.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn create_courses(
	map_id: MapID,
	courses: &[NewCourse],
	txn: &mut Transaction<'_, MySql>,
) -> Result<Vec<CreatedCourse>>
{
	let course_ids = QueryBuilder::new(queries::INSERT_COURSES)
		.tap_mut(|query| {
			query.push_values(courses.iter().enumerate(), |mut query, (idx, course)| {
				if let Some(name) = course.name.as_deref() {
					query.push_bind(name);
				} else {
					query.push_bind(format!("Course {}", idx + 1));
				}

				query.push_bind(course.description.as_deref());
				query.push_bind(map_id);
			});

			query.push(" RETURNING id ");
		})
		.build_query_scalar::<CourseID>()
		.fetch_all(txn.as_mut())
		.await?;

	let mut created_courses = Vec::with_capacity(courses.len());

	for (id, course) in iter::zip(course_ids, courses) {
		create_course_mappers(id, &course.mappers, txn).await?;
		create_course_filters(id, &course.filters, txn)
			.await?
			.pipe(|filter_ids| created_courses.push(CreatedCourse { id, filter_ids }));
	}

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%map_id,
		?created_courses,
		"created map courses",
	};

	Ok(created_courses)
}

/// Inserts course mappers into the database.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn create_course_mappers(
	course_id: CourseID,
	mapper_ids: &[SteamID],
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	QueryBuilder::new(queries::INSERT_COURSE_MAPPERS)
		.tap_mut(|query| {
			query.push_values(mapper_ids, |mut query, steam_id| {
				query.push_bind(course_id).push_bind(steam_id);
			});
		})
		.build()
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("player_id") {
				Error::MapperDoesNotExist
			} else {
				Error::Database(error)
			}
		})?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?mapper_ids,
		"created course mappers",
	};

	Ok(())
}

/// Inserts submitted course filters into the database and returns the generated
/// filter IDs.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn create_course_filters(
	course_id: CourseID,
	filters: &[NewFilter; 4],
	txn: &mut Transaction<'_, MySql>,
) -> Result<[FilterID; 4]>
{
	let filter_ids = QueryBuilder::new(queries::INSERT_COURSE_FILTERS)
		.tap_mut(|query| {
			query.push_values(filters, |mut query, filter| {
				query.push_bind(course_id);
				query.push_bind(filter.mode);
				query.push_bind(filter.teleports);
				query.push_bind(filter.tier);
				query.push_bind(filter.ranked_status);
				query.push_bind(filter.notes.as_deref());
			});

			query.push(" RETURNING id ");
		})
		.build_query_scalar()
		.fetch_all(txn.as_mut())
		.await?
		.try_conv::<[FilterID; 4]>()
		.expect("exactly 4 filters");

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?filter_ids,
		"created course filters",
	};

	Ok(filter_ids)
}

/// Updates a map's metadata.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn update_metadata(req: &UpdateMapRequest, txn: &mut Transaction<'_, MySql>) -> Result<()>
{
	if req.description.is_none() && req.workshop_id.is_none() && req.global_status.is_none() {
		return Ok(());
	}

	let update_query_result = sqlx::query! {
		r"
		UPDATE
		  Maps
		SET
		  description = COALESCE(?, description),
		  workshop_id = COALESCE(?, workshop_id),
		  global_status = COALESCE(?, global_status)
		WHERE
		  id = ?
		",
		req.description,
		req.workshop_id,
		req.global_status,
		req.map_id,
	}
	.execute(txn.as_mut())
	.await?;

	match update_query_result.rows_affected() {
		0 => return Err(Error::MapDoesNotExist),
		n => assert_eq!(n, 1, "updated more than 1 map"),
	}

	tracing::info!(target: "cs2kz_api::audit_log", map_id = %req.map_id, "updated map metadata");

	Ok(())
}

/// Checks Steam to see if a map's name or checksum have changed.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn check_steam(
	req: &UpdateMapRequest,
	txn: &mut Transaction<'_, MySql>,
	steam_svc: &SteamService,
) -> Result<()>
{
	if !req.check_steam && req.workshop_id.is_none() {
		return Ok(());
	}

	let workshop_id = match req.workshop_id {
		Some(id) => id,
		None => {
			sqlx::query_scalar! {
				r"
				SELECT
				  workshop_id `workshop_id: WorkshopID`
				FROM
				  Maps
				WHERE
				  id = ?
				",
				req.map_id,
			}
			.fetch_one(txn.as_mut())
			.await?
		}
	};

	let (map_name, checksum) = tokio::try_join! {
		steam_svc.fetch_map_name(workshop_id).map_err(Error::Steam),

		// TODO: put this in background task and return `202`?
		steam_svc.download_map(workshop_id)
			.map_err(Error::Steam)
			.and_then(|map_file| {
				map_file.checksum()
					.map_ok(Checksum::from)
					.map_err(Error::CalculateMapChecksum)
			}),
	}?;

	let query_result = sqlx::query! {
		r"
		UPDATE
		  Maps
		SET
		  name = ?,
		  checksum = ?
		WHERE
		  id = ?
		",
		map_name,
		checksum,
		req.map_id,
	}
	.execute(txn.as_mut())
	.await?;

	match query_result.rows_affected() {
		0 => return Err(Error::MapDoesNotExist),
		n => assert_eq!(n, 1, "updated more than 1 map"),
	}

	tracing::info! {
		target: "cs2kz_api::audit_log",
		map_id = %req.map_id,
		%map_name,
		%checksum,
		"updated map name and checksum",
	};

	Ok(())
}

/// Deletes mappers for a map from the database.
///
/// If `mapper_ids` contains **all** the mappers associated with this map, this
/// function will return an error. Every map must have at least one mapper at
/// all times!
///
/// # Panics
///
/// This function will panic if `mapper_ids` is empty.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn remove_mappers(
	map_id: MapID,
	mapper_ids: &[SteamID],
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	// This should be handled by the deserialization logic, but a sanity check never
	// hurt anyone before.
	assert!(!mapper_ids.is_empty(), "cannot remove 0 mappers");

	QueryBuilder::new("DELETE FROM Mappers WHERE map_id = ")
		.tap_mut(|query| {
			query.push_bind(map_id);
			query.push(" AND player_id IN (");

			query.separated(", ").pipe(|mut query| {
				for &steam_id in mapper_ids {
					query.push_bind(steam_id);
				}
			});

			query.push(")");
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	let remaining_mappers = sqlx::query_scalar! {
		r"
		SELECT
		  count(map_id)
		FROM
		  Mappers
		WHERE
		  map_id = ?
		",
		map_id,
	}
	.fetch_one(txn.as_mut())
	.await?;

	if remaining_mappers == 0 {
		return Err(Error::MapMustHaveMappers);
	}

	tracing::info!(target: "cs2kz_api::audit_log", %map_id, ?mapper_ids, "removed mappers");

	Ok(())
}

/// Applies course updates for a given map.
///
/// The provided pairs of `course_id->course_update` **must** belong to the
/// given `map_id`. If course updates unrelated to the `map_id` are submitted,
/// they will be rejected!
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(updates, txn))]
async fn update_courses(
	map_id: MapID,
	updates: impl IntoIterator<Item = (CourseID, CourseUpdate), IntoIter: Send> + Send,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Vec<UpdatedCourse>>
{
	let mut known_course_ids = sqlx::query_scalar! {
		r"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  map_id = ?
		",
		map_id,
	}
	.fetch(txn.as_mut())
	.try_collect::<HashSet<_>>()
	.await?;

	let mut updated_courses = Vec::with_capacity(known_course_ids.len());

	let updates = updates.into_iter().map(|(course_id, update)| {
		if known_course_ids.remove(&course_id) {
			Ok((course_id, update))
		} else {
			Err(Error::MismatchingCourseID { map_id, course_id })
		}
	});

	for update in updates {
		let (course_id, update) = update?;

		if let Some(course) = update_course(course_id, update, txn).await? {
			updated_courses.push(course);
		}
	}

	updated_courses.sort_unstable();

	tracing::info! {
		target: "cs2kz_api::audit_log",
		%map_id,
		?updated_courses,
		"updated courses",
	};

	Ok(updated_courses)
}

/// Applies a single course update.
///
/// This function will return `None` if the given `update` is empty, and no
/// database operations have actually been executed.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn update_course(
	course_id: CourseID,
	update: CourseUpdate,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Option<UpdatedCourse>>
{
	if update.is_empty() {
		return Ok(None);
	}

	if update.name.is_some() || update.description.is_some() {
		sqlx::query! {
			r"
			UPDATE
			  Courses
			SET
			  name = COALESCE(?, name),
			  description = COALESCE(?, description)
			WHERE
			  id = ?
			",
			update.name,
			update.description,
			course_id,
		}
		.execute(txn.as_mut())
		.await?;
	}

	if let Some(mappers) = update.added_mappers {
		create_course_mappers(course_id, &mappers, txn).await?;
	}

	if let Some(mappers) = update.removed_mappers {
		remove_course_mappers(course_id, &mappers, txn).await?;
	}

	let mut course = UpdatedCourse { id: course_id, updated_filter_ids: Vec::new() };

	if let Some(updates) = update.filter_updates.filter(|update| !update.is_empty()) {
		course.updated_filter_ids = update_filters(course_id, updates, txn).await?;
	}

	Ok(Some(course))
}

/// Deletes mappers for a course from the database.
///
/// If `mapper_ids` contains **all** the mappers associated with this course,
/// this function will return an error. Every course must have at least one
/// mapper at all times!
///
/// # Panics
///
/// This function will panic if `mapper_ids` is empty.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn remove_course_mappers(
	course_id: CourseID,
	mapper_ids: &[SteamID],
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	// This should be handled by the deserialization logic, but a sanity check never
	// hurt anyone before.
	assert!(!mapper_ids.is_empty(), "cannot remove 0 mappers");

	QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ")
		.tap_mut(|query| {
			query.push_bind(course_id);
			query.push(" AND player_id IN (");

			query.separated(", ").pipe(|mut query| {
				for &steam_id in mapper_ids {
					query.push_bind(steam_id);
				}
			});

			query.push(")");
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	let remaining_mappers = sqlx::query_scalar! {
		r"
		SELECT
		  count(course_id)
		FROM
		  CourseMappers
		WHERE
		  course_id = ?
		",
		course_id,
	}
	.fetch_one(txn.as_mut())
	.await?;

	if remaining_mappers == 0 {
		return Err(Error::CourseMustHaveMappers { course_id: Some(course_id) });
	}

	tracing::info!(target: "cs2kz_api::audit_log", %course_id, ?mapper_ids, "removed course mappers");

	Ok(())
}

/// Applies filter updates for a given course.
///
/// The provided pairs of `filter_id->filter_update` **must** belong to the
/// given `course_id`. If course updates unrelated to the `course_id` are
/// submitted, they will be rejected!
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(updates, txn))]
async fn update_filters(
	course_id: CourseID,
	updates: impl IntoIterator<Item = (FilterID, FilterUpdate), IntoIter: Send> + Send,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Vec<FilterID>>
{
	let mut known_filter_ids = sqlx::query_scalar! {
		r"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  course_id = ?
		",
		course_id,
	}
	.fetch(txn.as_mut())
	.try_collect::<HashSet<_>>()
	.await?;

	let mut updated_filter_ids = Vec::with_capacity(known_filter_ids.len());

	let updates = updates.into_iter().map(|(filter_id, update)| {
		if known_filter_ids.remove(&filter_id) {
			Ok((filter_id, update))
		} else {
			Err(Error::MismatchingFilterID { course_id, filter_id })
		}
	});

	for update in updates {
		let (filter_id, update) = update?;

		if let Some(filter_id) = update_filter(filter_id, update, txn).await? {
			updated_filter_ids.push(filter_id);
		}
	}

	updated_filter_ids.sort_unstable();

	tracing::info! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?updated_filter_ids,
		"updated course filters",
	};

	Ok(updated_filter_ids)
}

/// Applies a single filter update.
///
/// This function will return `None` if the given `update` is empty, and no
/// database operations have actually been executed.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"), skip(txn))]
async fn update_filter(
	filter_id: FilterID,
	update: FilterUpdate,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Option<FilterID>>
{
	if update.is_empty() {
		return Ok(None);
	}

	sqlx::query! {
		r"
		UPDATE
		  CourseFilters
		SET
		  tier = COALESCE(?, tier),
		  ranked_status = COALESCE(?, ranked_status),
		  notes = COALESCE(?, notes)
		WHERE
		  id = ?
		",
		update.tier,
		update.ranked_status,
		update.notes,
		filter_id,
	}
	.execute(txn.as_mut())
	.await?;

	Ok(Some(filter_id))
}

#[cfg(test)]
mod tests
{
	use std::collections::BTreeMap;

	use cs2kz::{GlobalStatus, Mode, RankedStatus, Tier};
	use sqlx::{MySql, Pool};

	use super::*;
	use crate::testing::{self, ALPHAKEKS_ID};

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/checkmate.sql")
	)]
	async fn fetch_map_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapRequest { ident: "checkmate".parse()? };
		let res = svc.fetch_map(req).await?;

		testing::assert!(res.is_some());
		testing::assert_matches!(res.as_ref().map(|r| &*r.name), Some("kz_checkmate"));

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_map_not_found(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapRequest { ident: "foobar".parse()? };
		let res = svc.fetch_map(req).await?;

		testing::assert!(res.is_none());

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn fetch_maps_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapsRequest::default();
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 2);
		testing::assert_eq!(res.total, 2);

		for found in ["kz_checkmate", "kz_grotto"]
			.iter()
			.map(|name| res.maps.iter().find(|m| &m.name == name))
		{
			testing::assert!(found.is_some());
		}

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn fetch_maps_works_with_name(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapsRequest { name: Some(String::from("checkmate")), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert_eq!(res.maps[0].name, "kz_checkmate");

		let req = FetchMapsRequest { name: Some(String::from("grotto")), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert_eq!(res.maps[0].name, "kz_grotto");

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/checkmate.sql")
	)]
	async fn fetch_maps_works_with_workshop_id(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapsRequest { workshop_id: Some(3070194623.into()), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert_eq!(res.maps[0].name, "kz_checkmate");

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn fetch_maps_works_with_global_status(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req =
			FetchMapsRequest { global_status: Some(GlobalStatus::Global), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert_eq!(res.maps[0].name, "kz_checkmate");

		let req =
			FetchMapsRequest { global_status: Some(GlobalStatus::InTesting), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 1);
		testing::assert_eq!(res.total, 1);
		testing::assert_eq!(res.maps[0].name, "kz_grotto");

		let req =
			FetchMapsRequest { global_status: Some(GlobalStatus::NotGlobal), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 0);
		testing::assert_eq!(res.total, 0);

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn fetch_maps_works_with_limit(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapsRequest { limit: 1.into(), ..Default::default() };
		let res = svc.fetch_maps(req).await?;

		testing::assert_eq!(res.maps.len(), 1);
		testing::assert_eq!(res.total, 2);

		let found = ["kz_checkmate", "kz_grotto"]
			.iter()
			.filter_map(|name| res.maps.iter().find(|m| &m.name == name))
			.count();

		testing::assert_eq!(found, 1);

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn fetch_maps_works_with_offset(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let req = FetchMapsRequest::default();
		let all = svc.fetch_maps(req).await?;

		testing::assert_eq!(all.maps.len() as u64, all.total);

		let req = FetchMapsRequest { limit: 1.into(), offset: 0.into(), ..Default::default() };
		let first_two = svc.fetch_maps(req).await?;

		testing::assert_eq!(first_two.maps.len(), 1);
		testing::assert_eq!(first_two.total, 2);

		let req = FetchMapsRequest { limit: 1.into(), offset: 1.into(), ..Default::default() };
		let last_two = svc.fetch_maps(req).await?;

		testing::assert_eq!(first_two.maps.len(), 1);
		testing::assert_eq!(first_two.total, 2);

		let all = all.maps.into_iter();
		let chained = first_two.maps.into_iter().chain(last_two.maps);

		for (a, b) in iter::zip(all, chained) {
			testing::assert_eq!(a, b);
		}

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn create_map_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let mut txn = database.begin().await?;

		let req = SubmitMapRequest {
			workshop_id: 69.into(),
			description: None,
			global_status: GlobalStatus::InTesting,
			mappers: vec![ALPHAKEKS_ID],
			courses: vec![NewCourse {
				name: None,
				description: Some(String::from("course description!")),
				mappers: vec![ALPHAKEKS_ID],
				filters: [
					NewFilter {
						mode: Mode::Vanilla,
						teleports: true,
						tier: Tier::Medium,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Vanilla,
						teleports: false,
						tier: Tier::Hard,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Classic,
						teleports: true,
						tier: Tier::VeryEasy,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Classic,
						teleports: false,
						tier: Tier::Easy,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
				],
			}],
		};

		let map_id = create_map("kz_foobar", Checksum::new(b"foobar"), &req, &mut txn).await?;

		let map = sqlx::query! {
			r"
			SELECT
			  name `name!: String`,
			  workshop_id `workshop_id!: u32`,
			  description,
			  global_status `global_status!: GlobalStatus`
			FROM
			  Maps
			WHERE
			  id = ?
			",
			map_id,
		}
		.fetch_one(txn.as_mut())
		.await?;

		testing::assert_eq!(map.name, "kz_foobar");
		testing::assert_eq!(map.workshop_id, 69);
		testing::assert!(map.description.is_none());
		testing::assert_eq!(map.global_status, GlobalStatus::InTesting);

		let create_mappers_result = create_mappers(map_id, &req.mappers, &mut txn).await;

		testing::assert!(create_mappers_result.is_ok());

		let courses = create_courses(map_id, &req.courses, &mut txn).await?;

		testing::assert_eq!(courses.len(), 1);

		txn.commit().await?;

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn create_map_rejects_no_mappers(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);

		let req = SubmitMapRequest {
			workshop_id: 69.into(),
			description: None,
			global_status: GlobalStatus::InTesting,
			mappers: Vec::new(),
			courses: vec![NewCourse {
				name: None,
				description: Some(String::from("course description!")),
				mappers: vec![ALPHAKEKS_ID],
				filters: [
					NewFilter {
						mode: Mode::Vanilla,
						teleports: true,
						tier: Tier::Medium,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Vanilla,
						teleports: false,
						tier: Tier::Hard,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Classic,
						teleports: true,
						tier: Tier::VeryEasy,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Classic,
						teleports: false,
						tier: Tier::Easy,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
				],
			}],
		};

		let res = svc.submit_map(req).await.unwrap_err();

		testing::assert_matches!(res, Error::MapMustHaveMappers);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn create_map_rejects_no_courses(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);

		let req = SubmitMapRequest {
			workshop_id: 69.into(),
			description: None,
			global_status: GlobalStatus::InTesting,
			mappers: vec![ALPHAKEKS_ID],
			courses: Vec::new(),
		};

		let res = svc.submit_map(req).await.unwrap_err();

		testing::assert_matches!(res, Error::MapMustHaveCourses);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn create_map_rejects_no_course_mappers(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);

		let req = SubmitMapRequest {
			workshop_id: 69.into(),
			description: None,
			global_status: GlobalStatus::InTesting,
			mappers: vec![ALPHAKEKS_ID],
			courses: vec![NewCourse {
				name: None,
				description: Some(String::from("course description!")),
				mappers: Vec::new(),
				filters: [
					NewFilter {
						mode: Mode::Vanilla,
						teleports: true,
						tier: Tier::Medium,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Vanilla,
						teleports: false,
						tier: Tier::Hard,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Classic,
						teleports: true,
						tier: Tier::VeryEasy,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
					NewFilter {
						mode: Mode::Classic,
						teleports: false,
						tier: Tier::Easy,
						ranked_status: RankedStatus::Ranked,
						notes: None,
					},
				],
			}],
		};

		let res = svc.submit_map(req).await.unwrap_err();

		testing::assert_matches!(res, Error::CourseMustHaveMappers { .. });

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/checkmate.sql")
	)]
	async fn update_map_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let map_id = sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: MapID`
			FROM
			  Maps
			WHERE
			  name = "kz_checkmate"
			"#,
		}
		.fetch_one(&svc.database)
		.await?;

		let new_description = "a new description";
		let new_global_status = GlobalStatus::NotGlobal;
		let req = UpdateMapRequest {
			map_id,
			description: Some(String::from(new_description)),
			workshop_id: None,
			global_status: Some(new_global_status),
			check_steam: false,
			added_mappers: None,
			removed_mappers: None,
			course_updates: None,
		};

		let res = svc.update_map(req).await?;

		testing::assert!(res.updated_courses.is_empty());

		let map = sqlx::query! {
			r"
			SELECT
			  description,
			  global_status `global_status: GlobalStatus`
			FROM
			  Maps
			WHERE
			  id = ?
			",
			map_id,
		}
		.fetch_one(&svc.database)
		.await?;

		testing::assert_eq!(map.description.as_deref(), Some(new_description));
		testing::assert_eq!(map.global_status, new_global_status);

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn update_map_rejects_mismatching_course_id(
		database: Pool<MySql>,
	) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let (checkmate_id, grotto_course_id) = sqlx::query! {
			r#"
			SELECT
			  checkmate.id `checkmate_id: MapID`,
			  grotto.id `grotto_course_id: CourseID`
			FROM
			  Maps checkmate
			  JOIN Courses grotto
			WHERE
			  checkmate.name = "kz_checkmate"
			  AND grotto.map_id = (
			    SELECT
			      id
			    FROM
			      Maps
			    WHERE
			      name = "kz_grotto"
			  )
			"#,
		}
		.fetch_one(&svc.database)
		.await
		.map(|row| (row.checkmate_id, row.grotto_course_id))?;

		let new_description = "a new description";
		let new_global_status = GlobalStatus::NotGlobal;
		let req = UpdateMapRequest {
			map_id: checkmate_id,
			description: Some(String::from(new_description)),
			workshop_id: None,
			global_status: Some(new_global_status),
			check_steam: false,
			added_mappers: None,
			removed_mappers: None,
			course_updates: Some(BTreeMap::from_iter([(grotto_course_id, CourseUpdate {
				name: Some(String::from("this won't work!")),
				..Default::default()
			})])),
		};

		let res = svc.update_map(req).await.unwrap_err();

		testing::assert_matches!(
			res,
			Error::MismatchingCourseID { map_id, course_id }
				if map_id == checkmate_id && course_id == grotto_course_id
		);

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn update_map_rejects_mismatching_filter_id(
		database: Pool<MySql>,
	) -> color_eyre::Result<()>
	{
		let svc = testing::map_svc(database);
		let (map_id, course_id, filter_id) = sqlx::query! {
			r#"
			SELECT
			  m.id `map_id: MapID`,
			  c.id `course_id: CourseID`,
			  f.id `filter_id: FilterID`
			FROM
			  Maps m
			  JOIN Courses c ON c.map_id = m.id
			  JOIN CourseFilters f ON f.course_id = (
			    SELECT
			      id
			    FROM
			      Courses
			    WHERE
			      map_id = (
				SELECT
				  id
				FROM
				  Maps
				WHERE
				  name = "kz_grotto"
			      )
			  )
			WHERE
			  m.name = "kz_checkmate"
			"#,
		}
		.fetch_one(&svc.database)
		.await
		.map(|row| (row.map_id, row.course_id, row.filter_id))?;

		let new_description = "a new description";
		let new_global_status = GlobalStatus::NotGlobal;
		let req = UpdateMapRequest {
			map_id,
			description: Some(String::from(new_description)),
			workshop_id: None,
			global_status: Some(new_global_status),
			check_steam: false,
			added_mappers: None,
			removed_mappers: None,
			course_updates: Some(BTreeMap::from_iter([(course_id, CourseUpdate {
				name: Some(String::from("this won't work!")),
				filter_updates: Some(BTreeMap::from_iter([(filter_id, FilterUpdate {
					tier: Some(Tier::Impossible),
					..Default::default()
				})])),
				..Default::default()
			})])),
		};

		let res = svc.update_map(req).await.unwrap_err();

		testing::assert_matches!(
			res,
			Error::MismatchingFilterID { course_id: c_id, filter_id: f_id }
				if c_id == course_id && f_id == filter_id
		);

		Ok(())
	}
}
