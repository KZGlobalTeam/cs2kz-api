use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;
use std::{cmp, iter};

use axum::extract::{FromRef, State};
use axum::handler::Handler;
use axum::response::NoContent;
use axum::routing::{MethodRouter, Router};
use cs2kz::Context;
use cs2kz::checksum::Checksum;
use cs2kz::maps::courses::filters::{CourseFilterState, Tier};
use cs2kz::maps::{ApproveMapError, CourseId, MapId, MapState, UpdateMapError};
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::players::{CreatePlayerError, PlayerId};
use cs2kz::steam::WorkshopId;
use cs2kz::time::Timestamp;
use cs2kz::users::Permission;
use futures_util::{FutureExt, StreamExt, TryFutureExt, TryStreamExt, stream};

use crate::config::{CookieConfig, DepotDownloaderConfig, SteamAuthConfig};
use crate::extract::{Json, Path, Query};
use crate::middleware::auth::session_auth;
use crate::middleware::auth::session_auth::authorization::HasPermissions;
use crate::players::PlayerInfo;
use crate::response::{Created, ErrorResponse};
use crate::steam;

mod map_identifier;
pub use map_identifier::MapIdentifier;

#[derive(Clone)]
struct ApproveMapState {
    cx: Context,
    http_client: reqwest::Client,
    steam_auth_config: Arc<SteamAuthConfig>,
    depot_downloader_config: Arc<DepotDownloaderConfig>,
}

pub fn router<S>(
    cx: Context,
    cookie_config: impl Into<Arc<CookieConfig>>,
    steam_auth_config: impl Into<Arc<SteamAuthConfig>>,
    depot_downloader_config: impl Into<Arc<DepotDownloaderConfig>>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let session_auth_state = session_auth::State::new(cx.clone(), cookie_config)
        .authorize_with(HasPermissions::new(Permission::MapPool));
    let is_admin = axum::middleware::from_fn_with_state(session_auth_state, session_auth);
    let approve_map_state = ApproveMapState {
        cx,
        http_client: reqwest::Client::new(),
        steam_auth_config: steam_auth_config.into(),
        depot_downloader_config: depot_downloader_config.into(),
    };

    Router::new()
        .route(
            "/",
            MethodRouter::new()
                .put(approve_map.layer(is_admin.clone()))
                .with_state(approve_map_state.clone())
                .get(get_maps),
        )
        .route(
            "/{map}",
            MethodRouter::new()
                .patch(update_map.layer(is_admin))
                .with_state(approve_map_state)
                .get(get_map),
        )
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Map {
    /// The map's ID in the API.
    #[schema(value_type = u16, minimum = 1)]
    id: MapId,

    /// The map's ID on the Steam Workshop.
    #[schema(value_type = u32)]
    workshop_id: WorkshopId,

    /// The map's name.
    name: String,

    /// A brief description of the map.
    description: Option<String>,

    /// The state the map is currently in.
    #[schema(value_type = crate::openapi::shims::MapState)]
    state: MapState,

    /// A checksum of the map's `.vpk` file.
    #[schema(value_type = str)]
    vpk_checksum: Checksum,

    /// A list of players who have contributed to the creation of this map.
    mappers: Vec<PlayerInfo>,

    /// A list of courses present on the map.
    courses: Vec<Course>,

    /// When this map was approved.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    approved_at: Timestamp,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MapInfo {
    /// The map's ID.
    #[schema(value_type = u16, minimum = 1)]
    pub(crate) id: MapId,

    /// The map's name.
    pub(crate) name: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Course {
    /// The course's name.
    name: String,

    /// A brief description of the course.
    description: Option<String>,

    /// A list of players who have contributed to the creation of this course.
    mappers: Vec<PlayerInfo>,

    /// The filters for this course.
    filters: CourseFilters,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CourseInfo {
    /// The course's ID.
    #[schema(value_type = u16, minimum = 1)]
    pub(crate) id: CourseId,

    /// The course's name.
    pub(crate) name: String,

    #[schema(value_type = crate::openapi::shims::CourseFilterTier)]
    pub(crate) nub_tier: Tier,

    #[schema(value_type = crate::openapi::shims::CourseFilterTier)]
    pub(crate) pro_tier: Tier,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CourseFilters {
    /// The filter for the VNL mode.
    #[serde(deserialize_with = "deserialize_course_filter")]
    vanilla: CourseFilter,

    /// The filter for the CKZ mode.
    #[serde(deserialize_with = "deserialize_course_filter")]
    classic: CourseFilter,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CourseFilter {
    /// The difficulty level of this filter when teleports are allowed.
    #[schema(value_type = crate::openapi::shims::CourseFilterTier)]
    nub_tier: Tier,

    /// The difficulty level of this filter when no teleports are allowed.
    #[schema(value_type = crate::openapi::shims::CourseFilterTier)]
    pro_tier: Tier,

    /// The initial state the course should be in.
    #[schema(value_type = crate::openapi::shims::CourseFilterState)]
    state: CourseFilterState,

    /// Any additional notes on this filter (e.g. tiering justifications).
    #[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
    notes: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewMap {
    /// The ID of the map's Steam workshop item.
    #[schema(value_type = u32)]
    workshop_id: WorkshopId,

    /// A brief description of the map.
    #[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
    description: Option<String>,

    /// The initial state the map should be in.
    #[schema(value_type = crate::openapi::shims::MapState)]
    state: MapState,

    /// A list of SteamIDs of players who have contributed to the creation of this map.
    ///
    /// You must specify at least 1 player.
    #[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
    #[schema(value_type = Vec<crate::openapi::shims::SteamId>)]
    mappers: Vec<PlayerId>,

    /// A list of courses present on the map.
    ///
    /// You must specify at least 1 course.
    #[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
    courses: Vec<NewCourse>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewCourse {
    /// The course's name.
    ///
    /// This has to be unique across all courses belonging to this map.
    #[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
    name: String,

    /// A brief description of the course.
    #[serde(default, deserialize_with = "crate::serde::deserialize_empty_as_none")]
    description: Option<String>,

    /// A list of SteamIDs of players who have contributed to the creation of this course.
    ///
    /// You must specify at least 1 player.
    #[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
    #[schema(value_type = Vec<crate::openapi::shims::SteamId>)]
    mappers: Vec<PlayerId>,

    /// The filters for this course.
    filters: CourseFilters,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ApprovedMap {
    #[schema(value_type = u16, minimum = 1)]
    map_id: MapId,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct MapUpdate {
    /// A new workshop ID.
    ///
    /// This field is used for updating 3 things:
    ///    - the map's workshop ID
    ///    - the map's name
    ///    - the map's vpk checksum
    #[schema(value_type = Option<u32>)]
    workshop_id: Option<WorkshopId>,

    /// A new description.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    description: Option<String>,

    /// A new state.
    #[schema(value_type = Option<crate::openapi::shims::MapState>)]
    state: Option<MapState>,

    /// SteamIDs to add to the map's mapper list.
    #[serde(default)]
    #[schema(value_type = Vec<crate::openapi::shims::SteamId>)]
    added_mappers: Vec<PlayerId>,

    /// SteamIDs to remove from the map's mapper list.
    #[serde(default)]
    #[schema(value_type = Vec<crate::openapi::shims::SteamId>)]
    deleted_mappers: Vec<PlayerId>,

    /// Updates to individual courses.
    #[serde(default, deserialize_with = "deserialize_course_updates")]
    course_updates: Vec<CourseUpdate>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CourseUpdate {
    /// The index of the course to update.
    ///
    /// Courses are 1-indexed and always returned in-order by the API.
    idx: usize,

    /// A new name.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    name: Option<String>,

    /// A new description.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    description: Option<String>,

    /// SteamIDs to add to the course's mapper list.
    #[serde(default)]
    #[schema(value_type = Vec<crate::openapi::shims::SteamId>)]
    added_mappers: Vec<PlayerId>,

    /// SteamIDs to remove from the course's mapper list.
    #[serde(default)]
    #[schema(value_type = Vec<crate::openapi::shims::SteamId>)]
    deleted_mappers: Vec<PlayerId>,

    /// Updates to the course's filters.
    #[serde(default)]
    filter_updates: FilterUpdates,
}

#[derive(Debug, Default, serde::Deserialize, utoipa::ToSchema)]
pub struct FilterUpdates {
    vanilla: Option<FilterUpdate>,
    classic: Option<FilterUpdate>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct FilterUpdate {
    /// A new tier for records with teleports.
    #[schema(value_type = Option<crate::openapi::shims::CourseFilterTier>)]
    nub_tier: Option<Tier>,

    /// A new tier for records without teleports.
    #[schema(value_type = Option<crate::openapi::shims::CourseFilterTier>)]
    pro_tier: Option<Tier>,

    /// A new state.
    #[schema(value_type = Option<crate::openapi::shims::CourseFilterState>)]
    state: Option<CourseFilterState>,

    /// New notes.
    ///
    /// If you specify this, the old notes will be **replaced**!
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    notes: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetMapsQuery {
    /// Only include maps with this Steam Workshop ID.
    ///
    /// As multiple versions of the same map are represented as different maps, a request may
    /// return multiple values (multiple versions of the same map with the same workshop ID).
    #[param(value_type = Option<u32>)]
    workshop_id: Option<WorkshopId>,

    /// Only include maps whose name matches this value.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    name: Option<String>,

    /// Only include maps currently in this state.
    #[param(value_type = Option<crate::openapi::shims::MapState>)]
    state: Option<MapState>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<1000, 100>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

/// Approves a new map.
#[tracing::instrument(skip(cx, http_client), ret(level = "debug"))]
#[utoipa::path(
    put,
    path = "/maps",
    tag = "Maps",
    request_body = NewMap,
    responses(
        (status = 201, body = ApprovedMap),
        (status = 401,),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn approve_map(
    State(ApproveMapState {
        cx,
        http_client,
        steam_auth_config,
        depot_downloader_config,
    }): State<ApproveMapState>,
    Json(NewMap {
        workshop_id,
        description,
        state,
        mappers,
        courses,
    }): Json<NewMap>,
) -> Result<Created<ApprovedMap>, ErrorResponse> {
    let (name, vpk_checksum) =
        fetch_name_and_checksum(&http_client, &depot_downloader_config, workshop_id).await?;

    create_missing_mappers(
        &cx,
        &http_client,
        &steam_auth_config,
        iter::chain(&mappers, courses.iter().flat_map(|course| &course.mappers))
            .copied()
            .collect::<HashSet<_>>(),
    )
    .await?;

    let map = cs2kz::maps::NewMap {
        workshop_id,
        name,
        description,
        state,
        vpk_checksum,
        mappers: mappers.into_boxed_slice(),
        courses: courses
            .into_iter()
            .map(|course| cs2kz::maps::NewCourse {
                name: course.name,
                description: course.description,
                mappers: course.mappers.into_boxed_slice(),
                filters: cs2kz::maps::NewCourseFilters {
                    vanilla: cs2kz::maps::NewCourseFilter {
                        nub_tier: course.filters.vanilla.nub_tier,
                        pro_tier: course.filters.vanilla.pro_tier,
                        state: course.filters.vanilla.state,
                        notes: course.filters.vanilla.notes,
                    },
                    classic: cs2kz::maps::NewCourseFilter {
                        nub_tier: course.filters.classic.nub_tier,
                        pro_tier: course.filters.classic.pro_tier,
                        state: course.filters.classic.state,
                        notes: course.filters.classic.notes,
                    },
                },
            })
            .collect(),
    };

    cs2kz::maps::approve(&cx, map)
        .await
        .map(|map_id| Created(ApprovedMap { map_id }))
        .map_err(|err| match err {
            ApproveMapError::Database(error) => ErrorResponse::internal_server_error(error),
        })
}

/// Returns the latest KZ maps.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/maps",
    tag = "Maps",
    params(GetMapsQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<Map>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_maps(
    State(cx): State<Context>,
    Query(GetMapsQuery { workshop_id, name, state, limit, offset }): Query<GetMapsQuery>,
) -> Result<Json<Paginated<Vec<Map>>>, ErrorResponse> {
    let params = cs2kz::maps::GetMapsParams {
        workshop_id,
        name: name.as_deref(),
        state,
        limit,
        offset,
    };

    let maps = cs2kz::maps::get(&cx, params)
        .map_ok(Paginated::map_into)
        .and_then(Paginated::collect)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .await?;

    Ok(Json(maps))
}

/// Returns the map with the specified ID / name.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/maps/{map}",
    tag = "Maps",
    params(("map" = MapIdentifier, Path, description = "a map ID or name")),
    responses(
        (status = 200, body = Map),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_map(
    State(cx): State<Context>,
    Path(map_identifier): Path<MapIdentifier>,
) -> Result<Json<Map>, ErrorResponse> {
    let map = match map_identifier {
        MapIdentifier::Id(id) => cs2kz::maps::get_by_id(&cx, id).await,
        MapIdentifier::Name(ref name) => cs2kz::maps::get_by_name(&cx, name).try_next().await,
    }
    .map_err(|err| ErrorResponse::internal_server_error(err))?
    .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(map.into()))
}

/// Updates a map in-place.
///
/// This endpoint is used for simple metadata changes. Gameplay changes should be communicated
/// through a separate version, i.e. `PUT /maps`.
#[tracing::instrument(skip(cx, http_client))]
#[utoipa::path(
    patch,
    path = "/maps/{map_id}",
    tag = "Maps",
    params(("map_id" = u16, Path, description = "the map's ID")),
    request_body = MapUpdate,
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 409,),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn update_map(
    State(ApproveMapState {
        cx,
        http_client,
        steam_auth_config,
        depot_downloader_config,
    }): State<ApproveMapState>,
    Path(map_id): Path<MapId>,
    Json(MapUpdate {
        workshop_id,
        description,
        state,
        added_mappers,
        deleted_mappers,
        course_updates,
    }): Json<MapUpdate>,
) -> Result<NoContent, ErrorResponse> {
    let name_and_checksum = if let Some(workshop_id) = workshop_id {
        Some(fetch_name_and_checksum(&http_client, &depot_downloader_config, workshop_id).await?)
    } else {
        None
    };

    let has_mappers = !added_mappers.is_empty()
        || course_updates
            .iter()
            .any(|course| !course.added_mappers.is_empty());

    if has_mappers {
        create_missing_mappers(
            &cx,
            &http_client,
            &steam_auth_config,
            iter::chain(
                &added_mappers,
                course_updates
                    .iter()
                    .flat_map(|update| &update.added_mappers),
            )
            .copied()
            .collect::<HashSet<_>>(),
        )
        .await?;
    }

    let update = cs2kz::maps::MapUpdate {
        id: map_id,
        workshop_id,
        name: name_and_checksum.as_ref().map(|(name, _)| name.as_str()),
        description: description.as_deref(),
        state,
        vpk_checksum: name_and_checksum.as_ref().map(|&(_, checksum)| checksum),
        added_mappers: &added_mappers,
        deleted_mappers: &deleted_mappers,
        course_updates: course_updates
            .iter()
            .map(|update| cs2kz::maps::CourseUpdate {
                idx: update.idx,
                name: update.name.as_deref(),
                description: update.description.as_deref(),
                added_mappers: &update.added_mappers,
                deleted_mappers: &update.deleted_mappers,
                filter_updates: cs2kz::maps::FilterUpdates {
                    vanilla: update.filter_updates.vanilla.as_ref().map(|filter_update| {
                        cs2kz::maps::FilterUpdate {
                            nub_tier: filter_update.nub_tier,
                            pro_tier: filter_update.pro_tier,
                            state: filter_update.state,
                            notes: filter_update.notes.as_deref(),
                        }
                    }),
                    classic: update.filter_updates.classic.as_ref().map(|filter_update| {
                        cs2kz::maps::FilterUpdate {
                            nub_tier: filter_update.nub_tier,
                            pro_tier: filter_update.pro_tier,
                            state: filter_update.state,
                            notes: filter_update.notes.as_deref(),
                        }
                    }),
                },
            })
            .collect(),
    };

    match cs2kz::maps::update(&cx, update).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(UpdateMapError::MustHaveMappers) => Err(ErrorResponse::map_must_have_mappers()),
        Err(UpdateMapError::InvalidCourseIndex { idx }) => {
            Err(ErrorResponse::invalid_course_index(idx))
        },
        Err(UpdateMapError::Database(error)) => Err(ErrorResponse::internal_server_error(error)),
    }
}

async fn fetch_name_and_checksum(
    http_client: &reqwest::Client,
    depot_downloader_config: &DepotDownloaderConfig,
    workshop_id: WorkshopId,
) -> Result<(String, Checksum), ErrorResponse> {
    try_join!(
        steam::fetch_map_name(http_client, workshop_id).map(|res| match res {
            Ok(Some(map_name)) => Ok(map_name),
            Ok(None) => Err(ErrorResponse::not_found()),
            Err(error) => Err(ErrorResponse::from(error)),
        }),
        steam::download_map(
            workshop_id,
            &depot_downloader_config.exe_path,
            &depot_downloader_config.out_dir
        )
        .and_then(|()| steam::maps::compute_checksum(depot_downloader_config.vpk_path(workshop_id)))
        .map_err(|err| ErrorResponse::internal_server_error(err)),
    )
}

async fn create_missing_mappers(
    cx: &Context,
    http_client: &reqwest::Client,
    steam_auth_config: &SteamAuthConfig,
    mapper_ids: impl IntoIterator<Item = PlayerId>,
) -> Result<(), ErrorResponse> {
    let players = stream::iter(mapper_ids)
        .then(|mapper_id| {
            steam::fetch_user(http_client, &steam_auth_config.web_api_key, mapper_id.into()).map(
                |user| match user {
                    Ok(Some(user)) => Ok(cs2kz::players::NewPlayer {
                        id: PlayerId::new(user.id),
                        name: Cow::Owned(user.name),
                        ip_address: None,
                    }),
                    Ok(None) => Err(ErrorResponse::mapper_does_not_exist()),
                    Err(error) => Err(ErrorResponse::from(error)),
                },
            )
        })
        .try_collect::<Vec<_>>()
        .await?;

    match cs2kz::players::create_many(cx, players).await {
        Ok(()) | Err(CreatePlayerError::PlayerAlreadyExists) => Ok(()),
        Err(CreatePlayerError::Database(error)) => Err(ErrorResponse::internal_server_error(error)),
    }
}

impl From<cs2kz::maps::Map> for Map {
    fn from(map: cs2kz::maps::Map) -> Self {
        Self {
            id: map.id,
            workshop_id: map.workshop_id,
            name: map.name,
            description: map.description,
            state: map.state,
            vpk_checksum: map.vpk_checksum,
            mappers: map.mappers.into_iter().map(PlayerInfo::from).collect(),
            courses: map.courses.into_iter().map(Course::from).collect(),
            approved_at: map.approved_at,
        }
    }
}

impl From<cs2kz::maps::MapInfo> for MapInfo {
    fn from(map: cs2kz::maps::MapInfo) -> Self {
        Self { id: map.id, name: map.name }
    }
}

impl From<cs2kz::maps::CourseInfo> for CourseInfo {
    fn from(course: cs2kz::maps::CourseInfo) -> Self {
        Self {
            id: course.id,
            name: course.name,
            nub_tier: course.nub_tier,
            pro_tier: course.pro_tier,
        }
    }
}

impl From<cs2kz::maps::Course> for Course {
    fn from(course: cs2kz::maps::Course) -> Self {
        Self {
            name: course.name,
            description: course.description,
            mappers: course.mappers.into_iter().map(PlayerInfo::from).collect(),
            filters: CourseFilters::from(course.filters),
        }
    }
}

impl From<cs2kz::maps::CourseFilters> for CourseFilters {
    fn from(filters: cs2kz::maps::CourseFilters) -> Self {
        Self {
            vanilla: CourseFilter::from(filters.vanilla),
            classic: CourseFilter::from(filters.classic),
        }
    }
}

impl From<cs2kz::maps::CourseFilter> for CourseFilter {
    fn from(filter: cs2kz::maps::CourseFilter) -> Self {
        Self {
            nub_tier: filter.nub_tier,
            pro_tier: filter.pro_tier,
            state: filter.state,
            notes: filter.notes,
        }
    }
}

fn deserialize_course_filter<'de, D>(deserializer: D) -> Result<CourseFilter, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let filter = <CourseFilter as serde::Deserialize<'de>>::deserialize(deserializer)?;

    if !cmp::min(filter.nub_tier, filter.pro_tier).is_humanly_possible() && filter.state.is_ranked()
    {
        return Err(serde::de::Error::custom(
            "filter cannot be ranked if its lowest tier is higher than 8",
        ));
    }

    Ok(filter)
}

fn deserialize_course_updates<'de, D>(deserializer: D) -> Result<Vec<CourseUpdate>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let updates = <Vec<CourseUpdate> as serde::Deserialize<'de>>::deserialize(deserializer)?;
    let mut indices = HashSet::new();

    if let Some(idx) = updates
        .iter()
        .map(|update| update.idx)
        .find(|&idx| !indices.insert(idx))
    {
        return Err(serde::de::Error::custom(format_args!("duplicate update for course #{idx}")));
    }

    Ok(updates)
}
