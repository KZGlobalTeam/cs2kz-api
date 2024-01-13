use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use cs2kz::{Mode, SteamID, Tier};
use sqlx::{MySql, MySqlExecutor, QueryBuilder, Transaction};
use tracing::warn;

use crate::database::{GlobalStatus, RankedStatus};
use crate::extractors::State;
use crate::maps::models::NewCourse;
use crate::maps::{CreatedMap, MappersTable, NewMap};
use crate::responses::Created;
use crate::steam::workshop;
use crate::{responses, Error, Result};

/// Approve a new map or update an existing one with breaking changes.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Maps",
  path = "/maps",
  request_body = NewMap,
  responses(
    responses::Created<CreatedMap>,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["maps_approve"]),
  ),
)]
pub async fn create(
	state: State,
	Json(mut map): Json<NewMap>,
) -> Result<Created<Json<CreatedMap>>> {
	if map.mappers.is_empty() {
		return Err(Error::NoMappers);
	}

	map.courses.sort_by_key(|course| course.stage);

	let is_contiguous = map
		.courses
		.windows(2)
		.all(|courses| courses[0].stage + 1 == courses[1].stage);

	if !is_contiguous {
		return Err(Error::NonContiguousStages);
	}

	for course in &map.courses {
		validate_course(course)?;
	}

	let mut transaction = state.transaction().await?;

	let map_id = insert_map(&map, state.http(), &mut transaction).await?;

	insert_mappers(MappersTable::Map(map_id), &map.mappers, transaction.as_mut()).await?;
	insert_courses(map_id, &map.courses, &mut transaction).await?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedMap { map_id })))
}

fn validate_course(course: &NewCourse) -> Result<()> {
	if course.mappers.is_empty() {
		return Err(Error::NoCourseMappers { stage: course.stage });
	}

	const POSSIBLE_FILTERS: [(Mode, bool); 4] = [
		(Mode::Vanilla, true),
		(Mode::Vanilla, false),
		(Mode::Classic, true),
		(Mode::Classic, false),
	];

	if let Some((stage, mode, teleports)) = POSSIBLE_FILTERS
		.into_iter()
		.find(|&filter| {
			!course
				.filters
				.iter()
				.any(|f| filter == (f.mode, f.teleports))
		})
		.map(|(mode, teleports)| (course.stage, mode, teleports))
	{
		return Err(Error::MissingFilter { stage, mode, teleports });
	}

	if let Some((stage, mode, teleports)) = course
		.filters
		.iter()
		.find(|filter| filter.tier > Tier::Death && filter.ranked_status == RankedStatus::Ranked)
		.map(|filter| (course.stage, filter.mode, filter.teleports))
	{
		return Err(Error::UnrankableFilter { stage, mode, teleports });
	}

	Ok(())
}

async fn insert_map(
	map: &NewMap,
	http_client: Arc<reqwest::Client>,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<u16> {
	let workshop_id = map.workshop_id;
	let (workshop_map, checksum) = tokio::try_join! {
		workshop::Map::get(workshop_id, http_client),
		async { workshop::MapFile::download(workshop_id).await?.checksum().await },
	}?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  global_status = ?
		WHERE
		  name = ?
		"#,
		GlobalStatus::NotGlobal,
		workshop_map.name,
	}
	.execute(transaction.as_mut())
	.await?;

	let rows_affected = query_result.rows_affected();

	if rows_affected > 1 {
		warn! {
			amount = %rows_affected,
			%workshop_id,
			name = %workshop_map.name,
			%checksum,
			"degloballed more than 1 map",
		};
	}

	sqlx::query! {
		r#"
		INSERT INTO
		  Maps (name, workshop_id, checksum, global_status)
		VALUES
		  (?, ?, ?, ?)
		"#,
		workshop_map.name,
		map.workshop_id,
		checksum,
		map.global_status,
	}
	.execute(transaction.as_mut())
	.await?;

	sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)
		.map_err(Error::from)
}

pub(super) async fn insert_mappers(
	table: MappersTable,
	mappers: &[SteamID],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO");

	let table_id = match table {
		MappersTable::Map(map_id) => {
			query.push("Mappers (map_id, ");
			map_id as u32
		}
		MappersTable::Course(course_id) => {
			query.push("CourseMappers (course_id, ");
			course_id
		}
	};

	query.push("player_id)");
	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(table_id).push_bind(steam_id);
	});

	query.build().execute(executor).await?;

	Ok(())
}

async fn insert_courses(
	map_id: u16,
	courses: &[NewCourse],
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO Courses (map_id, map_stage, name)");

	query.push_values(courses, |mut query, course| {
		query
			.push_bind(map_id)
			.push_bind(course.stage)
			.push_bind(&course.name);
	});

	query.build().execute(transaction.as_mut()).await?;

	let course_ids = sqlx::query! {
		r#"
		SELECT
		  id,
		  map_stage
		FROM
		  Courses
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		    FROM
		      Courses
		  )
		"#,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.map(|row| (row.map_stage, row.id))
	.collect::<HashMap<u8, u32>>();

	for course in courses {
		let course_id = course_ids
			.get(&course.stage)
			.copied()
			.expect("we just inserted this");

		insert_course_details(course_id, course, transaction).await?;
	}

	Ok(())
}

async fn insert_course_details(
	course_id: u32,
	course: &NewCourse,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	let mut insert_mappers = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

	insert_mappers.push_values(&course.mappers, |mut query, steam_id| {
		query.push_bind(course_id).push_bind(steam_id);
	});

	insert_mappers.build().execute(transaction.as_mut()).await?;

	let mut insert_filters = QueryBuilder::new(
		r#"
		INSERT INTO
		  CourseFilters (
		    course_id,
		    mode_id,
		    teleports,
		    tier,
		    ranked_status
		  )
		"#,
	);

	insert_filters.push_values(&course.filters, |mut query, filter| {
		query
			.push_bind(course_id)
			.push_bind(filter.mode)
			.push_bind(filter.teleports)
			.push_bind(filter.tier)
			.push_bind(filter.ranked_status);
	});

	insert_filters.build().execute(transaction.as_mut()).await?;

	Ok(())
}
