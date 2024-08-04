//! A service for managing KZ maps.

use std::collections::HashSet;
use std::{fmt, iter};

use axum::extract::FromRef;
use cs2kz::{GlobalStatus, SteamID};
use futures::{TryFutureExt, TryStreamExt};
use itertools::Itertools;
use sqlx::{MySql, Pool, QueryBuilder, Transaction};
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
		.bind(req.name)
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
		",
		map_name,
		req.description,
		req.global_status,
		req.workshop_id,
		checksum,
	}
	.execute(txn.as_mut())
	.await?
	.last_insert_id()
	.try_conv::<MapID>()
	.expect("in-range ID");

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
	QueryBuilder::new(queries::INSERT_COURSES)
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
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	let course_ids = sqlx::query_scalar! {
		r"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		",
	}
	.fetch_all(txn.as_mut())
	.await?;

	let mut created_courses = Vec::with_capacity(courses.len());

	for (id, course) in iter::zip(course_ids, courses) {
		create_course_mappers(id, &course.mappers, txn).await?;
		create_course_filters(id, &course.filters, txn)
			.await?
			.pipe(|filter_ids| created_courses.push(CreatedCourse { id, filter_ids }));
	}

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
	QueryBuilder::new(queries::INSERT_COURSE_FILTERS)
		.tap_mut(|query| {
			query.push_values(filters, |mut query, filter| {
				query.push_bind(course_id);
				query.push_bind(filter.mode);
				query.push_bind(filter.teleports);
				query.push_bind(filter.tier);
				query.push_bind(filter.ranked_status);
				query.push_bind(filter.notes.as_deref());
			});
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	let filter_ids = sqlx::query_scalar! {
		r"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		",
	}
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
		return Err(Error::CourseMustHaveMappers { course_id });
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
