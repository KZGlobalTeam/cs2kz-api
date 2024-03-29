//! Handlers for the `/maps/{map}` route.

use std::num::{NonZeroU16, NonZeroU32};

use axum::extract::Path;
use axum::Json;
use cs2kz::{GlobalStatus, MapIdentifier, SteamID};
use sqlx::{MySql, QueryBuilder, Transaction};
use tracing::info;

use super::root::create_mappers;
use crate::auth::RoleFlags;
use crate::maps::handlers::root::insert_course_mappers;
use crate::maps::models::CourseUpdate;
use crate::maps::{queries, FullMap, MapUpdate};
use crate::responses::NoContent;
use crate::sqlx::UpdateQuery;
use crate::workshop::WorkshopMap;
use crate::{auth, responses, AppState, Error, Result};

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/maps/{map}",
  tag = "Maps",
  params(MapIdentifier),
  responses(
    responses::Ok<FullMap>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: AppState, Path(map): Path<MapIdentifier>) -> Result<Json<FullMap>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE ");

	match map {
		MapIdentifier::ID(id) => {
			query.push(" m.id = ").push_bind(id);
		}
		MapIdentifier::Name(name) => {
			query.push(" m.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	query.push(" ORDER BY m.id DESC ");

	let map = query
		.build_query_as::<FullMap>()
		.fetch_all(&state.database)
		.await?
		.into_iter()
		.reduce(FullMap::reduce)
		.ok_or(Error::no_content())?;

	Ok(Json(map))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  path = "/maps/{map_id}",
  tag = "Maps",
  security(("Browser Session" = ["maps"])),
  params(("map_id" = u16, Path, description = "The map's ID")),
  responses(//
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn patch(
	state: AppState,
	session: auth::Session<auth::HasRoles<{ RoleFlags::MAPS.as_u32() }>>,
	Path(map_id): Path<NonZeroU16>,
	Json(MapUpdate {
		description,
		workshop_id,
		global_status,
		check_steam,
		added_mappers,
		removed_mappers,
		course_updates,
	}): Json<MapUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.database.begin().await?;

	update_details(map_id, description, workshop_id, global_status, &mut transaction).await?;

	if check_steam || workshop_id.is_some() {
		update_name_and_checksum(map_id, &state.config, &state.http_client, &mut transaction)
			.await?;
	}

	if let Some(added_mappers) = added_mappers {
		create_mappers(map_id, &added_mappers, &mut transaction).await?;
	}

	if let Some(removed_mappers) = removed_mappers {
		delete_mappers(map_id, &removed_mappers, &mut transaction).await?;
	}

	if let Some(course_updates) = course_updates {
		update_courses(map_id, course_updates, &mut transaction).await?;
	}

	transaction.commit().await?;

	Ok(NoContent)
}

/// Updates map details.
async fn update_details(
	map_id: NonZeroU16,
	description: Option<String>,
	workshop_id: Option<u32>,
	global_status: Option<GlobalStatus>,
	transaction: &mut Transaction<'_, MySql>,
) -> Result<()> {
	if description.is_none() && workshop_id.is_none() && global_status.is_none() {
		return Ok(());
	}

	let mut query = UpdateQuery::new("UPDATE Maps");

	if let Some(description) = description {
		query.set("description", description);
	}

	if let Some(workshop_id) = workshop_id {
		query.set("workshop_id", workshop_id);
	}

	if let Some(global_status) = global_status {
		query.set("global_status", global_status);
	}

	query.push(" WHERE id = ").push_bind(map_id.get());

	let query_result = query.build().execute(transaction.as_mut()).await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("map ID"));
	}

	info!(target: "audit_log", %map_id, "updated map details");

	Ok(())
}

/// Updates a map's name and checksum by downloading its map file from Steam.
async fn update_name_and_checksum(
	map_id: NonZeroU16,
	config: &crate::Config,
	http_client: &reqwest::Client,
	transaction: &mut Transaction<'_, MySql>,
) -> Result<()> {
	let workshop_id = sqlx::query!("SELECT workshop_id FROM Maps where id = ?", map_id.get())
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.workshop_id)?;

	let name = WorkshopMap::fetch_name(workshop_id, http_client).await?;
	let checksum = WorkshopMap::download(workshop_id, config)
		.await?
		.checksum()
		.await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  name = ?,
		  checksum = ?
		WHERE
		  id = ?
		"#,
		name,
		checksum,
		map_id.get(),
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("map ID"));
	}

	info!(target: "audit_log", %map_id, "updated workshop details");

	Ok(())
}

/// Deletes mappers from the database.
async fn delete_mappers(
	map_id: NonZeroU16,
	mappers: &[SteamID],
	transaction: &mut Transaction<'_, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("DELETE FROM Mappers WHERE map_id = ");

	query.push_bind(map_id.get()).push(" AND player_id IN (");

	let mut separated = query.separated(", ");

	for &steam_id in mappers {
		separated.push_bind(steam_id);
	}

	query.push(")");
	query.build().execute(transaction.as_mut()).await?;

	info!(target: "audit_log", %map_id, ?mappers, "removed mappers");

	let remaining_mappers = sqlx::query! {
		r#"
		SELECT
		  COUNT(map_id) count
		FROM
		  Mappers
		WHERE
		  map_id = ?
		"#,
		map_id.get(),
	}
	.fetch_one(transaction.as_mut())
	.await
	.map(|row| row.count)?;

	if remaining_mappers == 0 {
		return Err(Error::must_have_mappers());
	}

	Ok(())
}

/// Updates courses.
async fn update_courses<C>(
	map_id: NonZeroU16,
	courses: C,
	transaction: &mut Transaction<'_, MySql>,
) -> Result<()>
where
	C: IntoIterator<Item = (NonZeroU32, CourseUpdate)> + Send,
	C::IntoIter: Send,
{
	let mut course_ids = Vec::new();

	for (course_id, CourseUpdate { name, description, added_mappers, removed_mappers }) in courses {
		let is_empty_update = name.is_none()
			&& description.is_none()
			&& added_mappers.is_none()
			&& removed_mappers.is_none();

		if is_empty_update {
			continue;
		}

		if name.is_some() || description.is_some() {
			let mut query = UpdateQuery::new("UPDATE Courses");

			if let Some(name) = name {
				query.set("name", name);
			}

			if let Some(description) = description {
				query.set("description", description);
			}

			query.push(" WHERE id = ").push_bind(course_id.get());
			query.build().execute(transaction.as_mut()).await?;
		}

		if let Some(added_mappers) = added_mappers {
			insert_course_mappers(course_id, &added_mappers, transaction).await?;
		}

		if let Some(removed_mappers) = removed_mappers {
			delete_course_mappers(course_id, &removed_mappers, transaction).await?;
		}

		course_ids.push(course_id);
	}

	course_ids.sort_unstable();

	info!(target: "audit_log", %map_id, ?course_ids, "updated courses");

	Ok(())
}

/// Deletes course mappers from the database.
async fn delete_course_mappers(
	course_id: NonZeroU32,
	mappers: &[SteamID],
	transaction: &mut Transaction<'_, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ");

	query.push_bind(course_id.get()).push(" AND player_id IN (");

	let mut separated = query.separated(", ");

	for &steam_id in mappers {
		separated.push_bind(steam_id);
	}

	query.push(")");
	query.build().execute(transaction.as_mut()).await?;

	info!(target: "audit_log", %course_id, ?mappers, "deleted course mappers");

	let remaining_mappers = sqlx::query! {
		r#"
		SELECT
		  COUNT(course_id) count
		FROM
		  CourseMappers
		WHERE
		  course_id = ?
		"#,
		course_id.get(),
	}
	.fetch_one(transaction.as_mut())
	.await
	.map(|row| row.count)?;

	if remaining_mappers == 0 {
		return Err(Error::must_have_mappers());
	}

	Ok(())
}
