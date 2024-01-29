use std::collections::HashMap;

use axum::Json;
use cs2kz::{Mode, SteamID, Tier};
use sqlx::{MySql, MySqlExecutor, QueryBuilder, Transaction};

use crate::database::{GlobalStatus, RankedStatus};
use crate::maps::models::NewCourse;
use crate::maps::{CreatedMap, MappersTable, NewMap};
use crate::responses::Created;
use crate::sqlx::SqlErrorExt;
use crate::steam::workshop;
use crate::{audit, responses, AppState, Error, Result};

/// Approve a new map or update an existing one with breaking changes.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Maps",
  path = "/maps",
  request_body = NewMap,
  responses(
    responses::Created<CreatedMap>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["maps"]),
  ),
)]
pub async fn create(
	state: AppState,
	Json(mut map): Json<NewMap>,
) -> Result<Created<Json<CreatedMap>>> {
	if map.mappers.is_empty() {
		return Err(Error::NoMappers);
	}

	if map.courses.is_empty() {
		return Err(Error::NoCourses);
	}

	map.courses.sort_by_key(|course| course.stage);

	if !(1..=100).contains(&map.courses[0].stage) {
		return Err(Error::InvalidStage);
	}

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

	let map_id = insert_map(&map, state.http(), state.config(), &mut transaction).await?;

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
	http_client: &reqwest::Client,
	config: &crate::Config,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<u16> {
	let workshop_id = map.workshop_id;
	let (workshop_map, checksum) = tokio::try_join! {
		workshop::Map::get(workshop_id, http_client),
		async { workshop::MapFile::download(workshop_id, config).await?.checksum().await },
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

	if rows_affected > 0 {
		audit!("degloballed old map versions", map = %workshop_map.name);
	}

	if rows_affected > 1 {
		audit! {
			warn,
			"degloballed more than 1 map",
			amount = %rows_affected,
			%workshop_id,
			name = %workshop_map.name,
			%checksum
		};
	}

	sqlx::query! {
		r#"
		INSERT INTO
		  Maps (name, workshop_id, checksum, global_status, description)
		VALUES
		  (?, ?, ?, ?, ?)
		"#,
		workshop_map.name,
		map.workshop_id,
		checksum,
		map.global_status,
		if matches!(map.description.as_deref(), Some("")) {
			&None
		} else {
			&map.description
		},
	}
	.execute(transaction.as_mut())
	.await?;

	let map_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

	audit! {
		"created map",
		id = %map_id,
		name = %workshop_map.name,
		global_status = %map.global_status
	};

	Ok(map_id)
}

pub(super) async fn insert_mappers(
	table: MappersTable,
	mappers: &[SteamID],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	if mappers.is_empty() {
		return Ok(());
	}

	let mut query = QueryBuilder::new("INSERT INTO ");

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

	query.build().execute(executor).await.map_err(|err| {
		if err.is_foreign_key_violation_of("player_id") {
			Error::UnknownMapper
		} else {
			Error::MySql(err)
		}
	})?;

	audit!("created mappers", ?table, ?mappers);

	Ok(())
}

async fn insert_courses(
	map_id: u16,
	courses: &[NewCourse],
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO Courses (map_id, map_stage, name, description)");

	query.push_values(courses, |mut query, course| {
		query
			.push_bind(map_id)
			.push_bind(course.stage)
			.push_bind(if matches!(course.name.as_deref(), Some("")) {
				&None
			} else {
				&course.name
			})
			.push_bind(if matches!(course.description.as_deref(), Some("")) {
				&None
			} else {
				&course.description
			});
	});

	query.build().execute(transaction.as_mut()).await?;

	audit!("created courses", %map_id);

	let course_ids = sqlx::query! {
		r#"
		SELECT
		  id,
		  map_stage
		FROM
		  Courses
		WHERE
		  id >= (SELECT LAST_INSERT_ID())
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

	insert_mappers
		.build()
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_foreign_key_violation_of("player_id") {
				Error::UnknownMapper
			} else {
				Error::MySql(err)
			}
		})?;

	audit!("created mappers", %course_id);

	let mut insert_filters = QueryBuilder::new(
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

	insert_filters.push_values(&course.filters, |mut query, filter| {
		query
			.push_bind(course_id)
			.push_bind(filter.mode)
			.push_bind(filter.teleports)
			.push_bind(filter.tier)
			.push_bind(filter.ranked_status)
			.push_bind(if matches!(filter.notes.as_deref(), Some("")) {
				&None
			} else {
				&filter.notes
			});
	});

	insert_filters.build().execute(transaction.as_mut()).await?;

	audit!("created filters", %course_id);

	Ok(())
}
