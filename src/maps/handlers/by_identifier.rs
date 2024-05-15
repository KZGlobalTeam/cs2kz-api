//! Handlers for the `/maps/{map}` route.

use std::collections::HashSet;

use axum::extract::Path;
use axum::Json;
use cs2kz::{GlobalStatus, MapIdentifier, SteamID};
use futures::TryFutureExt;
use sqlx::{MySql, QueryBuilder};
use tracing::{debug, info};

use super::root::create_mappers;
use crate::authorization::{self, Permissions};
use crate::maps::handlers::root::insert_course_mappers;
use crate::maps::{
	queries, CourseID, CourseUpdate, FilterID, FilterUpdate, FullMap, MapID, MapUpdate,
};
use crate::openapi::responses;
use crate::openapi::responses::NoContent;
use crate::sqlx::UpdateQuery;
use crate::steam::workshop::{self, WorkshopID};
use crate::{authentication, Error, Result, State};

/// Fetch a single map.
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
pub async fn get(state: &State, Path(map): Path<MapIdentifier>) -> Result<Json<FullMap>> {
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
		.ok_or_else(|| Error::no_content())?;

	Ok(Json(map))
}

/// Update a specific map.
///
/// This endpoint is used for non-breaking changes, i.e. changes that do not change the
/// **gameplay** of a map in a backwards-incompatible way. This could include the map's name,
/// filters, mappers, etc.
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
	state: &State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::MAPS.value() }>>,
	Path(map_id): Path<MapID>,
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
	let mut transaction = state.transaction().await?;

	update_details(
		map_id,
		description,
		workshop_id,
		global_status,
		&mut transaction,
	)
	.await?;

	if check_steam || workshop_id.is_some() {
		update_name_and_checksum(
			map_id,
			workshop_id,
			&state.config,
			&state.http_client,
			&mut transaction,
		)
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

	info!(%map_id, "updated map");

	Ok(NoContent)
}

/// Updates map details.
async fn update_details(
	map_id: MapID,
	description: Option<String>,
	workshop_id: Option<WorkshopID>,
	global_status: Option<GlobalStatus>,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	if description.is_none() && workshop_id.is_none() && global_status.is_none() {
		return Ok(());
	}

	let mut query = UpdateQuery::new("Maps");

	if let Some(description) = description {
		query.set("description", description);
	}

	if let Some(workshop_id) = workshop_id {
		query.set("workshop_id", workshop_id);
	}

	if let Some(global_status) = global_status {
		query.set("global_status", global_status);
	}

	query.push(" WHERE id = ").push_bind(map_id);

	let query_result = query.build().execute(transaction.as_mut()).await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("map ID"));
	}

	debug!(target: "audit_log", %map_id, "updated map details");

	Ok(())
}

/// Updates a map's name and checksum by downloading its map file from Steam.
async fn update_name_and_checksum(
	map_id: MapID,
	workshop_id: Option<WorkshopID>,
	config: &crate::Config,
	http_client: &reqwest::Client,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	let workshop_id = if let Some(workshop_id) = workshop_id {
		workshop_id
	} else {
		sqlx::query_scalar! {
			r#"
			SELECT
			  workshop_id `workshop_id: WorkshopID`
			FROM
			  Maps
			WHERE
			  id = ?
			"#,
			map_id,
		}
		.fetch_one(transaction.as_mut())
		.await?
	};

	let (name, checksum) = tokio::try_join! {
		workshop::fetch_map_name(workshop_id, http_client),
		workshop::MapFile::download(workshop_id, config).and_then(|map| async move {
			map.checksum().await.map_err(|err| {
				Error::internal_server_error("failed to compute map checksum").with_source(err)
			})
		}),
	}?;

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
		map_id,
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("map ID"));
	}

	debug!(target: "audit_log", %map_id, "updated workshop details");

	Ok(())
}

/// Deletes mappers from the database.
async fn delete_mappers(
	map_id: MapID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("DELETE FROM Mappers WHERE map_id = ");

	query.push_bind(map_id).push(" AND player_id IN (");

	let mut separated = query.separated(", ");

	for &steam_id in mappers {
		separated.push_bind(steam_id);
	}

	query.push(")");
	query.build().execute(transaction.as_mut()).await?;

	let remaining_mappers = sqlx::query_scalar! {
		r#"
		SELECT
		  COUNT(map_id) count
		FROM
		  Mappers
		WHERE
		  map_id = ?
		"#,
		map_id,
	}
	.fetch_one(transaction.as_mut())
	.await?;

	if remaining_mappers == 0 {
		return Err(Error::must_have_mappers());
	}

	debug!(target: "audit_log", %map_id, ?mappers, "deleted mappers");

	Ok(())
}

/// Updates courses.
async fn update_courses<C>(
	map_id: MapID,
	courses: C,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
where
	C: IntoIterator<Item = (CourseID, CourseUpdate)> + Send,
	C::IntoIter: Send,
{
	let mut valid_course_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  map_id = ?
		"#,
		map_id,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.collect::<HashSet<_>>();

	let courses = courses.into_iter().map(|(id, update)| {
		if valid_course_ids.remove(&id) {
			(id, Ok(update))
		} else {
			(id, Err(Error::course_does_not_belong_to_map(id, map_id)))
		}
	});

	let mut updated_course_ids = Vec::new();

	for (course_id, update) in courses {
		if let Some(course_id) = update_course(map_id, course_id, update?, transaction).await? {
			updated_course_ids.push(course_id);
		}
	}

	updated_course_ids.sort_unstable();

	debug!(target: "audit_log", %map_id, ?updated_course_ids, "updated courses");

	Ok(())
}

/// Updates an individual course.
async fn update_course(
	map_id: MapID,
	course_id: CourseID,
	CourseUpdate {
		name,
		description,
		added_mappers,
		removed_mappers,
		filter_updates,
	}: CourseUpdate,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Option<CourseID>> {
	if name.is_none()
		&& description.is_none()
		&& added_mappers.is_none()
		&& removed_mappers.is_none()
		&& filter_updates.is_none()
	{
		return Ok(None);
	}

	if name.is_some() || description.is_some() {
		let mut query = UpdateQuery::new("Courses");

		if let Some(name) = name {
			query.set("name", name);
		}

		if let Some(description) = description {
			query.set("description", description);
		}

		query.push(" WHERE id = ").push_bind(course_id);
		query.build().execute(transaction.as_mut()).await?;
	}

	if let Some(added_mappers) = added_mappers {
		insert_course_mappers(course_id, &added_mappers, transaction).await?;
	}

	if let Some(removed_mappers) = removed_mappers {
		delete_course_mappers(course_id, &removed_mappers, transaction).await?;
	}

	if let Some(filter_updates) = filter_updates {
		update_filters(map_id, course_id, filter_updates, transaction).await?;
	}

	Ok(Some(course_id))
}

/// Deletes course mappers from the database.
async fn delete_course_mappers(
	course_id: CourseID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	let mut query = QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ");

	query.push_bind(course_id).push(" AND player_id IN (");

	let mut separated = query.separated(", ");

	for &steam_id in mappers {
		separated.push_bind(steam_id);
	}

	query.push(")");
	query.build().execute(transaction.as_mut()).await?;

	let remaining_mappers = sqlx::query_scalar! {
		r#"
		SELECT
		  COUNT(course_id) count
		FROM
		  CourseMappers
		WHERE
		  course_id = ?
		"#,
		course_id,
	}
	.fetch_one(transaction.as_mut())
	.await?;

	if remaining_mappers == 0 {
		return Err(Error::must_have_mappers());
	}

	debug!(target: "audit_log", %course_id, ?mappers, "deleted course mappers");

	Ok(())
}

/// Applies updates to filters for a given course.
async fn update_filters<F>(
	map_id: MapID,
	course_id: CourseID,
	filters: F,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
where
	F: IntoIterator<Item = (FilterID, FilterUpdate)> + Send,
	F::IntoIter: Send,
{
	let mut valid_filter_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  course_id = ?
		"#,
		course_id,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.collect::<HashSet<_>>();

	let filters = filters.into_iter().map(|(id, update)| {
		if valid_filter_ids.remove(&id) {
			(id, Ok(update))
		} else {
			(
				id,
				Err(Error::filter_does_not_belong_to_course(id, course_id)),
			)
		}
	});

	let mut updated_filter_ids = Vec::new();

	for (filter_id, update) in filters {
		if let Some(filter_id) = update_filter(filter_id, update?, transaction).await? {
			updated_filter_ids.push(filter_id);
		}
	}

	updated_filter_ids.sort_unstable();

	debug! {
		target: "audit_log",
		%map_id,
		course.id = %course_id,
		course.updated_filters = ?updated_filter_ids,
		"updated filters",
	};

	Ok(())
}

/// Updates information about a course filter.
async fn update_filter(
	filter_id: FilterID,
	FilterUpdate {
		tier,
		ranked_status,
		notes,
	}: FilterUpdate,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Option<FilterID>> {
	if tier.is_none() && ranked_status.is_none() && notes.is_none() {
		return Ok(None);
	}

	let mut query = UpdateQuery::new("CourseFilters");

	if let Some(tier) = tier {
		query.set("tier", tier);
	}

	if let Some(ranked_status) = ranked_status {
		query.set("ranked_status", ranked_status);
	}

	if let Some(notes) = notes {
		query.set("notes", notes);
	}

	query.push(" WHERE id = ").push_bind(filter_id);
	query.build().execute(transaction.as_mut()).await?;

	Ok(Some(filter_id))
}
