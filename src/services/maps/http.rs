//! HTTP handlers for this service.

use std::collections::{BTreeMap, BTreeSet};

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::{GlobalStatus, SteamID};
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	Error,
	FetchMapRequest,
	FetchMapResponse,
	FetchMapsRequest,
	FetchMapsResponse,
	MapService,
	SubmitMapRequest,
	SubmitMapResponse,
	UpdateMapRequest,
	UpdateMapResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::session::{authorization, user, SessionManagerLayer};
use crate::services::auth::Session;
use crate::services::maps::{CourseID, CourseUpdate, MapID};
use crate::services::steam::WorkshopID;
use crate::util::MapIdentifier;

impl From<MapService> for Router
{
	fn from(svc: MapService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				authorization::RequiredPermissions(user::Permissions::MAPS),
			));

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:map", routing::get(get_single))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		let protected = Router::new()
			.route("/", routing::put(submit_map).route_layer(auth.clone()))
			.route("/:map", routing::patch(update_map).route_layer(auth.clone()))
			.route_layer(middleware::cors::dashboard([
				http::Method::OPTIONS,
				http::Method::PUT,
				http::Method::PATCH,
			]))
			.with_state(svc.clone());

		public.merge(protected)
	}
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/maps",
	tag = "Maps",
	operation_id = "get_maps",
	params(FetchMapsRequest)
)]
async fn get_many(
	State(svc): State<MapService>,
	Query(req): Query<FetchMapsRequest>,
) -> Result<FetchMapsResponse, ProblemDetails>
{
	let res = svc.fetch_maps(req).await?;

	if res.maps.is_empty() {
		Err(Error::NoData)?;
	}

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(put, path = "/maps", tag = "Maps")]
async fn submit_map(
	session: Session,
	State(svc): State<MapService>,
	Json(req): Json<SubmitMapRequest>,
) -> Result<SubmitMapResponse, ProblemDetails>
{
	let res = svc.submit_map(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/maps/{map}", tag = "Maps", operation_id = "get_map", params(
  ("map" = MapIdentifier, Path, description = "a map's ID or name"),
))]
async fn get_single(
	State(svc): State<MapService>,
	Path(ident): Path<MapIdentifier>,
) -> Result<FetchMapResponse, ProblemDetails>
{
	let req = FetchMapRequest { ident };
	let res = svc.fetch_map(req).await?.ok_or(Error::MapDoesNotExist)?;

	Ok(res)
}

/// Query parameters for `PATCH /maps/{map}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "UpdateMapRequest", example = json!({
  "description": "a new description",
  "check_steam": true,
  "course_updates": {
    "1": {
      "description": "the main course! yippie!",
      "filter_updates": {
        "1": {
          "notes": "this is really hard!"
        }
      }
    }
  }
}))]
#[doc(hidden)]
pub(crate) struct UpdateMapRequestPayload
{
	/// A new description.
	#[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
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

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  patch,
  path = "/maps/{map_id}",
  tag = "Maps",
  params(("map_id" = MapID, Path, description = "a map's ID")),
  security(("Browser Session" = ["maps"])),
)]
async fn update_map(
	session: Session,
	State(svc): State<MapService>,
	Path(map_id): Path<MapID>,
	Json(UpdateMapRequestPayload {
		description,
		workshop_id,
		global_status,
		check_steam,
		added_mappers,
		removed_mappers,
		course_updates,
	}): Json<UpdateMapRequestPayload>,
) -> Result<UpdateMapResponse, ProblemDetails>
{
	let req = UpdateMapRequest {
		map_id,
		description,
		workshop_id,
		global_status,
		check_steam,
		added_mappers,
		removed_mappers,
		course_updates,
	};

	let res = svc.update_map(req).await?;

	Ok(res)
}

#[cfg(test)]
mod tests
{
	use axum::extract::Request;
	use axum::handler::Handler;
	use sqlx::{MySql, Pool};
	use tower::Service;

	use super::*;
	use crate::testing;

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures(
			"../../../database/fixtures/checkmate.sql",
			"../../../database/fixtures/grotto.sql",
		)
	)]
	async fn get_many_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::map_svc(database);
		let handler = routing::get(get_many);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(Default::default())?;

		let res = handler.call(req, state).await;

		testing::assert_eq!(res.status(), http::StatusCode::OK);

		let res = testing::parse_body::<FetchMapsResponse>(res.into_body()).await?;

		testing::assert_eq!(res.maps.len(), 2);
		testing::assert_eq!(res.total, 2);

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/checkmate.sql")
	)]
	async fn get_single_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::map_svc(database);
		let mut handler = Router::new()
			.route("/:map", routing::get(get_single))
			.with_state(state);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/checkmate")
			.body(axum::body::Body::default())?;

		let res = handler.call(req).await?;

		testing::assert_eq!(res.status(), http::StatusCode::OK);

		let res = testing::parse_body::<FetchMapResponse>(res.into_body()).await?;

		testing::assert_eq!(res.name, "kz_checkmate");

		Ok(())
	}
}
