//! Handlers for the `/maps` route.

use std::iter;

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{GlobalStatus, SteamID};
use futures::TryFutureExt;
use serde::Deserialize;
use sqlx::{MySql, QueryBuilder};
use utoipa::IntoParams;

use crate::authorization::Permissions;
use crate::make_id::IntoID;
use crate::maps::{
	queries, CourseID, CreatedMap, FilterID, FullMap, MapID, NewCourse, NewFilter, NewMap,
};
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses;
use crate::openapi::responses::{Created, PaginationResponse};
use crate::sqlx::{query, FilteredQuery, SqlErrorExt};
use crate::steam::workshop::{self, WorkshopID};
use crate::{authentication, authorization, Error, Result, State};

/// Query parameters for `GET /maps`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by name.
	name: Option<String>,

	/// Filter by workshop ID.
	workshop_id: Option<WorkshopID>,

	/// Filter by global status.
	global_status: Option<GlobalStatus>,

	/// Filter by creation date.
	created_after: Option<DateTime<Utc>>,

	/// Filter by creation date.
	created_before: Option<DateTime<Utc>>,

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Fetch maps.
///
/// Any maps returned by this endpoint are currently, or have been previously, accepted into the
/// global map pool.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/maps",
  tag = "Maps",
  responses(
    responses::Ok<PaginationResponse<FullMap>>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &State,
	Query(GetParams {
		name,
		workshop_id,
		global_status,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<PaginationResponse<FullMap>>> {
	let mut query = FilteredQuery::new(queries::SELECT);
	let mut transaction = state.transaction().await?;

	if let Some(name) = name {
		query.filter(" m.name LIKE ", format!("%{name}%"));
	}

	if let Some(workshop_id) = workshop_id {
		query.filter(" m.workshop_id = ", workshop_id);
	}

	if let Some(global_status) = global_status {
		query.filter(" m.global_status = ", global_status);
	}

	if let Some(created_after) = created_after {
		query.filter(" m.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter(" m.created_on < ", created_before);
	}

	// not entirely sure if this is correct?
	if let offset @ 1.. = offset.0 {
		query.filter(" m.id > ", offset);
	}

	query.push(" ORDER BY m.id DESC ");

	let maps = query
		.build_query_as::<FullMap>()
		.fetch_all(transaction.as_mut())
		.await
		.map(|maps| FullMap::flatten(maps, limit.into()))?;

	if maps.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: maps,
	}))
}

/// Create / update a map.
///
/// This is used whenever a new map is approved, or an existing map receives breaking changes.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  path = "/maps",
  tag = "Maps",
  security(("Browser Session" = ["maps"])),
  request_body = NewMap,
  responses(
    responses::Created<CreatedMap>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn put(
	state: &State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::MAPS.value() }>>,
	Json(NewMap {
		workshop_id,
		description,
		global_status,
		mappers,
		courses,
	}): Json<NewMap>,
) -> Result<Created<Json<CreatedMap>>> {
	let (name, checksum) = tokio::try_join! {
		workshop::fetch_map_name(workshop_id, &state.http_client),
		workshop::MapFile::download(workshop_id, &state.config).and_then(|map| async move {
			map.checksum().await.map_err(|err| {
				Error::checksum(err).context(format!("workshop_id: {workshop_id}"))
			})
		}),
	}?;

	let mut transaction = state.transaction().await?;

	let map_id = create_map(
		name,
		description,
		global_status,
		workshop_id,
		checksum,
		&mut transaction,
	)
	.await?;

	create_mappers(map_id, &mappers, &mut transaction).await?;
	create_courses(map_id, &courses, &mut transaction).await?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedMap { map_id })))
}

/// Inserts a new map into the database and returns its ID.
async fn create_map(
	name: String,
	description: Option<String>,
	global_status: GlobalStatus,
	workshop_id: WorkshopID,
	checksum: u32,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<MapID> {
	let deglobal_old_result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  global_status = -1
		WHERE
		  name = ?
		"#,
		name,
	}
	.execute(transaction.as_mut())
	.await?;

	match deglobal_old_result.rows_affected() {
		0 => {}
		1 => tracing::info! {
			target: "cs2kz_api::audit_log",
			%name,
			"degloballed old version of map",
		},
		amount => tracing::warn! {
			target: "cs2kz_api::audit_log",
			%name,
			%amount,
			"degloballed multiple versions of map",
		},
	}

	let map_id = sqlx::query! {
		r#"
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
		"#,
		name,
		description,
		global_status,
		workshop_id,
		checksum,
	}
	.execute(transaction.as_mut())
	.await?
	.last_insert_id()
	.into_id::<MapID>()?;

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, "created map");

	Ok(map_id)
}

/// Inserts mappers into the database.
pub(super) async fn create_mappers(
	map_id: MapID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO Mappers (map_id, player_id)");

	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(map_id).push_bind(steam_id);
	});

	query
		.build()
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::unknown("mapper").context(err)
			} else {
				Error::from(err)
			}
		})?;

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, ?mappers, "created mappers");

	Ok(())
}

/// Inserts map courses into the database.
async fn create_courses(
	map_id: MapID,
	courses: &[NewCourse],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Vec<CourseID>> {
	let mut query = QueryBuilder::new("INSERT INTO Courses (name, description, map_id)");

	query.push_values(courses, |mut query, course| {
		query
			.push_bind(course.name.as_deref())
			.push_bind(course.description.as_deref())
			.push_bind(map_id);
	});

	query.build().execute(transaction.as_mut()).await?;

	let course_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		"#,
	}
	.fetch_all(transaction.as_mut())
	.await?;

	for (&course_id, course) in iter::zip(&course_ids, courses) {
		insert_course_mappers(course_id, &course.mappers, transaction).await?;
		insert_course_filters(course_id, &course.filters, transaction).await?;
	}

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, ?course_ids, "created courses");

	Ok(course_ids)
}

/// Inserts mappers for a specific course into the database.
pub(super) async fn insert_course_mappers(
	course_id: CourseID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(course_id).push_bind(steam_id);
	});

	query
		.build()
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::unknown("mapper").context(err)
			} else {
				Error::from(err)
			}
		})?;

	tracing::debug!(target: "cs2kz_api::audit_log", %course_id, ?mappers, "created course mappers");

	Ok(())
}

/// Inserts course filters for a specific course into the database.
async fn insert_course_filters(
	course_id: CourseID,
	filters: &[NewFilter; 4],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Vec<FilterID>> {
	let mut query = QueryBuilder::new(
		r#"
		INSERT INTO
		  CourseFilters (
		    course_id,
		    mode_id,
		    teleports,
		    tier,
		    ranked_status,
		    notes
		  )
		"#,
	);

	query.push_values(filters, |mut query, filter| {
		query
			.push_bind(course_id)
			.push_bind(filter.mode)
			.push_bind(filter.teleports)
			.push_bind(filter.tier)
			.push_bind(filter.ranked_status)
			.push_bind(filter.notes.as_deref());
	});

	query.build().execute(transaction.as_mut()).await?;

	let filter_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		"#,
	}
	.fetch_all(transaction.as_mut())
	.await?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?filter_ids,
		"created course filters",
	};

	Ok(filter_ids)
}
