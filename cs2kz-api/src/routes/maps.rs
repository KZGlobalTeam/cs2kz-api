use {
	crate::{
		res::{maps as res, BadRequest},
		util::Created,
		Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{MapIdentifier, PlayerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	utoipa::{IntoParams, ToSchema},
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMapsParams<'a> {
	pub name: Option<String>,
	pub created_by: Option<PlayerIdentifier<'a>>,
	pub created_after: Option<DateTime<Utc>>,
	pub created_before: Option<DateTime<Utc>>,
	pub offset: Option<u64>,
	pub limit: Option<u64>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Maps", context_path = "/api/v0", path = "/maps", params(GetMapsParams), responses(
	(status = 200, body = Vec<KZMap>),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_maps(
	state: State,
	Query(GetMapsParams { name, created_by, created_after, created_before, offset, limit }): Query<
		GetMapsParams<'_>,
	>,
) -> Response<Vec<res::KZMap>> {
	todo!();
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Maps", context_path = "/api/v0", path = "/maps/{ident}", params(
	("ident" = MapIdentifier, Path, description = "The map's ID or name")
), responses(
	(status = 200, body = KZMap),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_map(state: State, Path(ident): Path<MapIdentifier<'_>>) -> Response<res::KZMap> {
	todo!();
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMap {
	name: String,
	workshop_id: Option<u32>,
	filters: Vec<Filter>,
	created_by: SteamID,
	approved_by: SteamID,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Filter {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMapWithId {
	id: u16,

	#[serde(flatten)]
	map: NewMap,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Maps", context_path = "/api/v0", path = "/maps", request_body = NewMap, responses(
	(status = 201, body = NewMapWithId),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn create_map(
	state: State,
	Json(NewMap { name, workshop_id, filters, created_by, approved_by }): Json<NewMap>,
) -> Result<Created<Json<NewMapWithId>>> {
	todo!();
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MapUpdate {
	name: Option<String>,
	workshop_id: Option<u32>,
	filters_added: Option<Vec<Filter>>,
	filters_removed: Option<Vec<u16>>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Maps", context_path = "/api/v0", path = "/maps/{id}", request_body = MapUpdate, responses(
	(status = 200),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn update_map(
	state: State,
	Json(MapUpdate { name, workshop_id, filters_added, filters_removed }): Json<MapUpdate>,
) -> Response<()> {
	todo!();
}
