use std::collections::HashSet;

use axum::extract::Path;
use axum::Json;
use cs2kz::{SteamID, Tier};
use itertools::Itertools;
use sqlx::{MySql, MySqlExecutor, QueryBuilder, Transaction};
use tracing::trace;

use crate::database::{GlobalStatus, RankedStatus};
use crate::extract::State;
use crate::maps::{CourseUpdate, FilterUpdate, MapUpdate, MappersTable};
use crate::steam::workshop;
use crate::{audit, query, responses, Error, Result};

/// Update a map with non-breaking changes.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Maps",
  path = "/maps/{map_id}",
  params(("map_id" = u16, Path, description = "The map's ID")),
  request_body = MapUpdate,
  responses(
    responses::Ok<()>,
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
pub async fn update(
	state: State,
	Path(map_id): Path<u16>,
	Json(map_update): Json<MapUpdate>,
) -> Result<()> {
	let mut transaction = state.transaction().await?;

	validate_update(map_id, &map_update, &mut transaction).await?;

	if let Some(global_status) = map_update.global_status {
		update_global_status(map_id, global_status, transaction.as_mut()).await?;
	}

	if let Some(description) = map_update.description {
		update_description(map_id, &description, transaction.as_mut()).await?;
	}

	let workshop_id = if let Some(workshop_id) = map_update.workshop_id {
		update_workshop_id(map_id, workshop_id, transaction.as_mut()).await?;
		workshop_id
	} else {
		sqlx::query!("SELECT workshop_id FROM Maps WHERE id = ?", map_id)
			.fetch_one(transaction.as_mut())
			.await?
			.workshop_id
	};

	if map_update.check_steam {
		update_name_and_checksum(
			map_id,
			workshop_id,
			state.http(),
			state.config(),
			transaction.as_mut(),
		)
		.await?;
	}

	if let Some(mappers) = &map_update.added_mappers {
		super::create::insert_mappers(MappersTable::Map(map_id), mappers, transaction.as_mut())
			.await?;
	}

	if let Some(mappers) = &map_update.removed_mappers {
		remove_mappers(MappersTable::Map(map_id), mappers, transaction.as_mut()).await?;
	}

	for course_update in map_update.course_updates.iter().flatten() {
		update_course(map_id, course_update, &mut transaction).await?;
	}

	transaction.commit().await?;

	Ok(())
}

async fn validate_update(
	map_id: u16,
	map_update: &MapUpdate,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	let course_ids = sqlx::query! {
		r#"
		SELECT
		  c.id
		FROM
		  Courses c
		  JOIN Maps m ON m.id = c.map_id
		WHERE
		  m.id = ?
		"#,
		map_id,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.map(|row| row.id)
	.collect::<HashSet<u32>>();

	if course_ids.is_empty() {
		return Err(Error::UnknownMapID(map_id));
	}

	if let Some(course_id) = map_update
		.course_updates
		.iter()
		.flatten()
		.map(|course| course.id)
		.find(|course_id| !course_ids.contains(course_id))
	{
		return Err(Error::InvalidCourse { map_id, course_id });
	}

	for course_update in map_update.course_updates.iter().flatten() {
		let filter_ids = sqlx::query! {
			r#"
			SELECT
			  f.id
			FROM
			  CourseFilters f
			  JOIN Courses c ON c.id = f.course_id
			WHERE
			  c.id = ?
			"#,
			course_update.id,
		}
		.fetch_all(transaction.as_mut())
		.await?;

		if let Some(filter) = course_update
			.filter_updates
			.iter()
			.flatten()
			.find(|filter| !filter_ids.iter().map(|row| row.id).contains(&filter.id))
		{
			return Err(Error::InvalidFilter { course_id: course_update.id, filter_id: filter.id });
		}
	}

	Ok(())
}

async fn update_global_status(
	map_id: u16,
	global_status: GlobalStatus,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  global_status = ?
		WHERE
		  id = ?
		"#,
		global_status,
		map_id,
	}
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::UnknownMapID(map_id));
	}

	audit!("updated global status for map", id = %map_id, %global_status);

	Ok(())
}

async fn update_description(
	map_id: u16,
	description: &str,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  description = ?
		WHERE
		  id = ?
		"#,
		description,
		map_id,
	}
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::UnknownMapID(map_id));
	}

	audit!("updated map description", id = %map_id, %description);

	Ok(())
}

async fn update_workshop_id(
	map_id: u16,
	workshop_id: u32,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  workshop_id = ?
		WHERE
		  id = ?
		"#,
		workshop_id,
		map_id,
	}
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::UnknownMapID(map_id));
	}

	audit!("updated workshop id", %map_id, %workshop_id);

	Ok(())
}

async fn update_name_and_checksum(
	map_id: u16,
	workshop_id: u32,
	http_client: &reqwest::Client,
	config: &crate::Config,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let (workshop_map, checksum) = tokio::try_join! {
		workshop::Map::get(workshop_id, http_client),
		async { workshop::MapFile::download(workshop_id, config).await?.checksum().await },
	}?;

	let result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  name = ?,
		  checksum = ?
		WHERE
		  id = ?
		"#,
		workshop_map.name,
		checksum,
		map_id,
	}
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::UnknownMapID(map_id));
	}

	trace! {
		id = %map_id,
		name = %workshop_map.name,
		%checksum,
		"updated workshop details for map",
	};

	Ok(())
}

async fn remove_mappers(
	table: MappersTable,
	mappers: &[SteamID],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	if mappers.is_empty() {
		return Ok(());
	}

	let mut query = QueryBuilder::new("DELETE FROM ");

	match table {
		MappersTable::Map(map_id) => {
			query.push("Mappers WHERE map_id = ").push_bind(map_id);
		}
		MappersTable::Course(course_id) => {
			query
				.push("CourseMappers WHERE course_id = ")
				.push_bind(course_id);
		}
	}

	query.push(" AND player_id IN ");
	query::push_tuple(mappers, &mut query);

	query.build().execute(executor).await?;

	Ok(())
}

async fn update_course(
	map_id: u16,
	update: &CourseUpdate,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	if let Some(mappers) = &update.added_mappers {
		super::create::insert_mappers(
			MappersTable::Course(update.id),
			mappers,
			transaction.as_mut(),
		)
		.await?;
	}

	if let Some(mappers) = &update.removed_mappers {
		remove_mappers(MappersTable::Course(update.id), mappers, transaction.as_mut()).await?;
	}

	if let Some(description) = update.description.as_deref() {
		let result = sqlx::query! {
			r#"
			UPDATE
			  Courses
			SET
			  description = ?
			WHERE
			  id = ?
			"#,
			description,
			update.id,
		}
		.execute(transaction.as_mut())
		.await?;

		if result.rows_affected() == 0 {
			return Err(Error::InvalidCourse { map_id, course_id: update.id });
		}

		audit!("updated course description", id = %update.id, %description);
	}

	for FilterUpdate { id, tier, ranked_status, notes } in update.filter_updates.iter().flatten() {
		if tier.is_none() && ranked_status.is_none() {
			continue;
		}

		if tier.is_some_and(|tier| tier > Tier::Death)
			&& matches!(ranked_status, Some(RankedStatus::Ranked))
		{
			return Err(Error::UnrankableFilterWithID { id: *id });
		}

		let mut query = QueryBuilder::new("UPDATE CourseFilters");
		let mut delimiter = " SET ";

		if let Some(tier) = tier {
			query.push(delimiter).push(" tier = ").push_bind(tier);

			delimiter = ",";
		}

		if let Some(ranked_status) = ranked_status {
			query
				.push(delimiter)
				.push(" ranked_status = ")
				.push_bind(ranked_status);

			delimiter = ",";
		}

		if let Some(notes) = notes.as_deref() {
			query.push(delimiter).push(" notes = ").push_bind(notes);
		}

		query.push(" WHERE id = ").push_bind(id);

		let result = query.build().execute(transaction.as_mut()).await?;

		if result.rows_affected() == 0 {
			return Err(Error::InvalidFilter { course_id: update.id, filter_id: *id });
		}

		audit!("updated filter", %id, ?tier, ?ranked_status, ?notes);
	}

	Ok(())
}
