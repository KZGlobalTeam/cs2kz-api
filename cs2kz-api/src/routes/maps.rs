//! This module holds all HTTP handlers related to maps.

use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use cs2kz::{MapIdentifier, Mode, PlayerIdentifier, SteamID, Tier};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, MySqlExecutor, QueryBuilder, Transaction};
use utoipa::{IntoParams, ToSchema};

use crate::models::maps::CreateCourseParams;
use crate::models::{KZMap, RankedStatus};
use crate::permissions::Permissions;
use crate::responses::Created;
use crate::sql::FetchID;
use crate::steam::WorkshopMap;
use crate::{audit, openapi as R, sql, AppState, Error, Result, State};

static GET_BASE_QUERY: &str = r#"
	SELECT
	  m.id,
	  m.workshop_id,
	  m.name,
	  p2.steam_id mapper_steam_id,
	  p2.name mapper_name,
	  c.id course_id,
	  c.map_stage course_stage,
	  p4.steam_id course_mapper_steam_id,
	  p4.name course_mapper_name,
	  f.id filter_id,
	  f.mode_id filter_mode,
	  f.teleports filter_teleports,
	  f.tier filter_tier,
	  f.ranked_status filter_ranked,
	  m.filesize,
	  m.created_on,
	  m.updated_on
	FROM
	  Maps m
	  JOIN Mappers p1 ON p1.map_id = m.id
	  JOIN Players p2 ON p2.steam_id = p1.player_id
	  JOIN Courses c ON c.map_id = m.id
	  JOIN CourseMappers p3 ON p3.course_id = c.id
	  JOIN Players p4 ON p4.steam_id = p3.player_id
	  JOIN CourseFilters f ON f.course_id = c.id
"#;

/// This function returns the router for the `/maps` routes.
pub fn router(state: &'static AppState) -> Router {
	let add_map = axum::middleware::from_fn_with_state(
		state,
		crate::middleware::auth::verify_web_user::<{ Permissions::MAPS_ADD.0 }>,
	);

	let edit_map = axum::middleware::from_fn_with_state(
		state,
		crate::middleware::auth::verify_web_user::<{ Permissions::MAPS_EDIT.0 }>,
	);

	Router::new()
		.route("/", get(get_maps))
		.route("/", post(create_map).layer(add_map))
		.route("/:ident", get(get_map_by_ident))
		.route("/:ident", patch(update_map).layer(edit_map))
		.route("/workshop/:id", get(get_map_by_workshop_id))
		.with_state(state)
}

/// This endpoint allows you to fetch globally approved maps.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Maps",
	path = "/maps",
	params(GetMapsParams),
	responses(
		R::Ok<KZMap>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_maps(
	state: State,
	Query(params): Query<GetMapsParams<'_>>,
) -> Result<Json<Vec<KZMap>>> {
	let mut query = QueryBuilder::new(GET_BASE_QUERY);
	let mut filter = sql::Filter::new();

	if let Some(name) = params.name {
		query
			.push(filter)
			.push(" m.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(player) = params.mapper {
		query.push(filter).push(
			r#"
			m.id IN (
			  SELECT
			    m1.id
			  FROM
			    Maps m1
			    JOIN Mappers m2 ON m2.map_id = m1.id
			  WHERE
			    m2.player_id =
			"#,
		);

		let steam_id = player.fetch_id(state.database()).await?;

		query.push_bind(steam_id).push(") ");
		filter.switch();
	}

	if let Some(created_after) = params.created_after {
		query
			.push(filter)
			.push(" m.created_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = params.created_before {
		query
			.push(filter)
			.push(" m.created_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	query.push(" ORDER BY m.id ASC ");

	sql::push_limits::<500>(params.limit, params.offset, &mut query);

	let maps = query
		.build_query_as::<KZMap>()
		.fetch_all(state.database())
		.await
		.map(KZMap::flatten)?;

	if maps.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(maps))
}

/// This endpoint is used for creating new maps.
///
/// It is intended to be used by admins and the map approval team.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Maps",
	path = "/maps",
	request_body = CreateMapRequest,
	responses(
		R::Created<CreateMapResponse>,
		R::NoContent,
		R::BadRequest,
		R::Conflict,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn create_map(
	state: State,
	Json(body): Json<CreateMapRequest>,
) -> Result<Created<Json<CreateMapResponse>>> {
	if body.mappers.is_empty() {
		return Err(Error::MissingMapField("mappers"));
	}

	validate_courses(&body.courses)?;

	let workshop_map = WorkshopMap::get(body.workshop_id, state.http_client())
		.await
		.ok_or(Error::InvalidWorkshopID(body.workshop_id))?;

	let mut transaction = state.begin_transaction().await?;

	sqlx::query! {
		r#"
		INSERT INTO
		  Maps (name, workshop_id, filesize)
		VALUES
		  (?, ?, ?)
		"#,
		workshop_map.name,
		body.workshop_id,
		workshop_map.filesize,
	}
	.execute(transaction.as_mut())
	.await?;

	let map_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await?
		.id as _;

	audit!(id = %map_id, %workshop_map.name, %body.workshop_id, "create map");

	create_mappers(map_id, &body.mappers, transaction.as_mut()).await?;
	create_courses(map_id, &body.courses, &mut transaction).await?;

	transaction.commit().await?;

	Ok(Created(Json(CreateMapResponse { map_id })))
}

/// This endpoint allows you to fetch a single map by its ID or (parts of its) name.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Maps",
	path = "/maps/{ident}",
	params(("ident" = MapIdentifier<'_>, Path, description = "The map's ID or name.")),
	responses(
		R::Ok<KZMap>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_map_by_ident(
	state: State,
	Path(ident): Path<MapIdentifier<'_>>,
) -> Result<Json<KZMap>> {
	let mut query = QueryBuilder::new(format!("{GET_BASE_QUERY} WHERE "));

	match ident {
		MapIdentifier::ID(id) => {
			query.push(" m.id = ").push_bind(id);
		}
		MapIdentifier::Name(name) => {
			query.push(" m.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	query
		.build_query_as::<KZMap>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.reduce(KZMap::reduce)
		.map(Json)
		.ok_or(Error::NoContent)
}

/// This endpoint allows you to fetch a map by its Steam Workshop ID.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Maps",
	path = "/maps/workshop/{id}",
	params(("id" = u32, Path, description = "The map's Steam Workshop ID.")),
	responses(
		R::Ok<KZMap>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_map_by_workshop_id(
	state: State,
	Path(workshop_id): Path<u32>,
) -> Result<Json<KZMap>> {
	sqlx::query_as(&format!("{GET_BASE_QUERY} WHERE m.workshop_id = ?"))
		.bind(workshop_id)
		.fetch_all(state.database())
		.await?
		.into_iter()
		.reduce(KZMap::reduce)
		.ok_or(Error::NoContent)
		.map(Json)
}

/// This endpoint is used for updating maps.
///
/// It is intended to be used by admins and the map approval team.
#[tracing::instrument]
#[utoipa::path(
	patch,
	tag = "Maps",
	path = "/maps/{id}",
	params(("id", Path, description = "The ID of the map you wish to update.")),
	request_body = UpdateMapRequest,
	responses(
		R::Ok,
		R::NoContent,
		R::BadRequest,
		R::Unauthorized,
		R::Conflict,
		R::InternalServerError,
	),
)]
pub async fn update_map(
	state: State,
	Path(map_id): Path<u16>,
	Json(body): Json<UpdateMapRequest>,
) -> Result<()> {
	if let Some(ref courses) = body.added_courses {
		validate_courses(courses)?;
	}

	let mut transaction = state.begin_transaction().await?;
	let mut update_map = QueryBuilder::new("UPDATE Maps");
	let mut delimiter = " SET ";

	let workshop_id = if let Some(workshop_id) = body.workshop_id {
		update_map
			.push(delimiter)
			.push(" workshop_id = ")
			.push_bind(workshop_id);

		delimiter = ",";
		workshop_id
	} else {
		sqlx::query!("SELECT workshop_id FROM Maps WHERE id = ?", map_id)
			.fetch_one(transaction.as_mut())
			.await?
			.workshop_id
	};

	if body.name || body.filesize {
		let workshop_map = WorkshopMap::get(workshop_id, state.http_client())
			.await
			.ok_or(Error::InvalidWorkshopID(workshop_id))?;

		if body.name {
			update_map
				.push(delimiter)
				.push(" name = ")
				.push_bind(workshop_map.name);

			delimiter = ",";
		}

		if body.filesize {
			update_map
				.push(delimiter)
				.push(" filesize = ")
				.push_bind(workshop_map.filesize);
		}
	}

	update_map.push(" WHERE id = ").push_bind(map_id);
	update_map.build().execute(transaction.as_mut()).await?;

	audit!(%map_id, update_name = %body.name, update_filesize = %body.filesize, "update map information");

	if let Some(mappers) = body.added_mappers {
		create_mappers(map_id, &mappers, transaction.as_mut()).await?;
	}

	if let Some(ref mappers) = body.removed_mappers {
		let mut remove_mappers = QueryBuilder::new("DELETE FROM Mappers WHERE player_id IN");

		remove_mappers.push_tuples(mappers, |mut query, steam_id| {
			query.push_bind(steam_id);
		});

		remove_mappers.build().execute(transaction.as_mut()).await?;

		audit!(?mappers, "remove mappers");
	}

	if let Some(ref courses) = body.added_courses {
		create_courses(map_id, courses, &mut transaction).await?;
	}

	if let Some(ref courses) = body.removed_courses {
		let mut remove_courses = QueryBuilder::new("DELETE FROM Courses WHERE id IN");

		remove_courses.push_tuples(courses, |mut query, course_id| {
			query.push_bind(course_id);
		});

		remove_courses.build().execute(transaction.as_mut()).await?;

		audit!(?courses, "remove courses");
	}

	for course_update in body.course_updates.iter().flatten() {
		if let Some(ref mappers) = course_update.added_mappers {
			let mappers = mappers
				.iter()
				.map(|&steam_id| CourseMapper { course_id: course_update.course_id, steam_id })
				.collect_vec();

			create_course_mappers(&mappers, transaction.as_mut()).await?;
		}

		if let Some(ref mappers) = course_update.removed_mappers {
			let mut remove_mappers =
				QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ");

			remove_mappers
				.push_bind(course_update.course_id)
				.push(" AND player_id IN ");

			remove_mappers.push_tuples(mappers, |mut query, &steam_id| {
				query.push_bind(steam_id);
			});

			remove_mappers.build().execute(transaction.as_mut()).await?;

			audit!(%course_update.course_id, ?mappers, "remove course mappers");
		}

		for filter in course_update.filter_updates.iter().flatten() {
			if filter.tier.is_none() && filter.ranked_status.is_none() {
				continue;
			}

			let mut update_filter = QueryBuilder::new("UPDATE CourseFilters");
			let mut delimiter = " SET ";

			if let Some(tier) = filter.tier {
				update_filter
					.push(delimiter)
					.push(" tier = ")
					.push_bind(tier);

				delimiter = ",";
			}

			if let Some(ranked_status) = filter.ranked_status {
				update_filter
					.push(delimiter)
					.push(" ranked_status = ")
					.push_bind(ranked_status);
			}

			update_filter
				.push(" WHERE id = ")
				.push_bind(filter.filter_id);

			update_filter.build().execute(transaction.as_mut()).await?;

			audit!(?filter, "update course filter");
		}
	}

	transaction.commit().await?;

	Ok(())
}

fn validate_courses(courses: &[CreateCourseParams]) -> Result<()> {
	if courses.is_empty() {
		return Err(Error::MissingMapField("courses"));
	}

	if courses.iter().any(|course| course.mappers.is_empty()) {
		return Err(Error::MissingMapField("courses.mappers"));
	}

	if courses.iter().any(|course| course.filters.len() != 4) {
		return Err(Error::MissingMapField("courses.filters"));
	}

	const POSSIBLE_FILTERS: [(Mode, bool); 4] = [
		(Mode::Vanilla, true),
		(Mode::Vanilla, false),
		(Mode::Classic, true),
		(Mode::Classic, false),
	];

	if let Some((mode, teleports)) = POSSIBLE_FILTERS.into_iter().find(|&filter| {
		!courses.iter().all(|course| {
			course
				.filters
				.iter()
				.any(|f| (f.mode, f.teleports) == filter)
		})
	}) {
		return Err(Error::MissingFilter { mode, teleports });
	}

	if let Some(tier) = courses.iter().find_map(|course| {
		course
			.filters
			.iter()
			.find(|filter| {
				filter.tier > Tier::Death && filter.ranked_status == RankedStatus::Ranked
			})
			.map(|filter| filter.tier)
	}) {
		return Err(Error::TooDifficultToRank { tier });
	}

	Ok(())
}

async fn create_mappers(
	map_id: u16,
	steam_ids: &[SteamID],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO Mappers (map_id, player_id)");

	query.push_values(steam_ids, |mut query, &steam_id| {
		query.push_bind(map_id).push_bind(steam_id);
	});

	query.build().execute(executor).await?;

	audit!(?steam_ids, "create mappers");

	Ok(())
}

async fn create_courses(
	map_id: u16,
	courses: &[CreateCourseParams],
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	let mut create_courses = QueryBuilder::new("INSERT INTO Courses (map_id, map_stage)");
	let stages = courses.iter().map(|course| course.stage).collect_vec();

	create_courses.push_values(&stages, |mut query, stage| {
		query.push_bind(map_id).push_bind(stage);
	});

	create_courses.build().execute(transaction.as_mut()).await?;

	audit!(%map_id, ?stages, "create courses");

	let first_course_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await?
		.id;

	let course_ids =
		sqlx::query!("SELECT id, map_stage FROM Courses WHERE id >= ?", first_course_id)
			.fetch_all(transaction.as_mut())
			.await?
			.into_iter()
			.map(|row| (row.map_stage, row.id))
			.collect::<HashMap<u8, u32>>();

	let course_mappers = courses
		.iter()
		.flat_map(|course| {
			let course_id = course_ids
				.get(&course.stage)
				.copied()
				.expect("we just inserted this");

			course
				.mappers
				.iter()
				.map(move |&steam_id| CourseMapper { course_id, steam_id })
		})
		.collect_vec();

	create_course_mappers(&course_mappers, transaction.as_mut()).await?;

	let mut create_course_filters = QueryBuilder::new(
		"INSERT INTO CourseFilters (course_id, mode_id, teleports, tier, ranked_status)",
	);

	let course_filters = courses
		.iter()
		.flat_map(|course| {
			let course_id = course_ids
				.get(&course.stage)
				.copied()
				.expect("we just inserted this");

			course.filters.iter().map(move |filter| (course_id, filter))
		})
		.collect_vec();

	create_course_filters.push_values(&course_filters, |mut query, (course_id, filter)| {
		query
			.push_bind(course_id)
			.push_bind(filter.mode)
			.push_bind(filter.teleports)
			.push_bind(filter.tier)
			.push_bind(filter.ranked_status);
	});

	create_course_filters
		.build()
		.execute(transaction.as_mut())
		.await?;

	audit!(?course_filters, "create course filters");

	Ok(())
}

#[derive(Debug)]
struct CourseMapper {
	course_id: u32,
	steam_id: SteamID,
}

async fn create_course_mappers(
	// NOTE(AlphaKeks): I really wanted to use `impl Iterator` here but that lead to weird
	// lifetime errors; probably an issue with `QueryBuilder::push_values`, but I don't know.
	mappers: &[CourseMapper],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

	query.push_values(mappers, |mut query, CourseMapper { course_id, steam_id }| {
		query.push_bind(course_id).push_bind(steam_id);
	});

	query.build().execute(executor).await?;

	audit!(?mappers, "create course mappers");

	Ok(())
}

/// Query parameters for retrieving information about maps.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMapsParams<'a> {
	/// The map's name.
	name: Option<String>,

	/// A player's SteamID or name.
	mapper: Option<PlayerIdentifier<'a>>,

	/// Only include maps created after this date.
	created_after: Option<DateTime<Utc>>,

	/// Only include maps created before this date.
	created_before: Option<DateTime<Utc>>,

	#[param(minimum = 0, maximum = 500)]
	limit: Option<u64>,
	offset: Option<i64>,
}

/// A new map.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "workshop_id": 3070194623_u32,
  "mappers": [
    "STEAM_1:0:102468802"
  ],
  "courses": [
    {
      "stage": 0,
      "mappers": [
        "STEAM_1:0:102468802"
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
  ]
}))]
pub struct CreateMapRequest {
	/// The map's Steam Workshop ID.
	workshop_id: u32,

	/// List of players who have contributed to creating this map.
	mappers: Vec<SteamID>,

	/// List of courses on this map.
	courses: Vec<CreateCourseParams>,
}

/// A map udpate.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({ "name": "kz_checkmate_v2_final_fix_global_new" }))]
pub struct UpdateMapRequest {
	/// The map's new Steam Workshop ID.
	workshop_id: Option<u32>,

	/// Update the map's name to the Workshop's version.
	#[serde(default)]
	name: bool,

	/// Update the map's filesize to the Workshop's value.
	#[serde(default)]
	filesize: bool,

	/// List of new mappers.
	added_mappers: Option<Vec<SteamID>>,

	/// List of old mappers to be removed.
	removed_mappers: Option<Vec<SteamID>>,

	/// List of new courses.
	added_courses: Option<Vec<CreateCourseParams>>,

	/// List of course IDs to be removed.
	removed_courses: Option<Vec<u32>>,

	/// List of updates to existing courses.
	course_updates: Option<Vec<CourseUpdate>>,
}

/// An update to a course.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "course_id": 1,
  "added_mappers": ["STEAM_1:0:102468802"]
}))]
pub struct CourseUpdate {
	/// The course's ID.
	course_id: u32,

	/// List of new mappers.
	added_mappers: Option<Vec<SteamID>>,

	/// List of old mappers to be removed.
	removed_mappers: Option<Vec<SteamID>>,

	/// List of updates for existing filters.
	filter_updates: Option<Vec<FilterUpdate>>,
}

/// An update to a filter.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "filter_id": 1,
  "tier": 7
}))]
pub struct FilterUpdate {
	/// The filter's ID.
	filter_id: u32,

	/// A different tier.
	tier: Option<Tier>,

	/// A new ranked status.
	ranked_status: Option<RankedStatus>,
}

/// A newly created map.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({ "map_id": 1 }))]
pub struct CreateMapResponse {
	/// The map's ID.
	map_id: u16,
}
