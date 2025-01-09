use axum::extract::{FromRef, State};
use axum::routing::{self, Router};
use cs2kz::Context;
use cs2kz::mode::Mode;
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::records::RecordId;
use cs2kz::styles::Styles;
use cs2kz::time::{Seconds, Timestamp};
use futures_util::{TryFutureExt, TryStreamExt};

use crate::extract::{Json, Path, Query};
use crate::maps::{CourseInfo, MapIdentifier, MapInfo};
use crate::players::{PlayerIdentifier, PlayerInfo};
use crate::replays::ReplayFile;
use crate::response::ErrorResponse;
use crate::servers::{ServerIdentifier, ServerInfo};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    Router::new()
        .route("/", routing::get(get_records))
        .route("/{record_id}", routing::get(get_record))
        .route("/{record_id}/replay", routing::get(get_record_replay))
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Record {
    #[schema(value_type = u32, minimum = 1)]
    id: RecordId,

    player: PlayerInfo,
    server: ServerInfo,
    map: MapInfo,
    course: CourseInfo,

    #[schema(value_type = crate::openapi::shims::Mode)]
    mode: Mode,

    #[schema(value_type = crate::openapi::shims::Styles)]
    styles: Styles,

    teleports: u32,

    /// Time in seconds.
    #[schema(value_type = f64)]
    time: Seconds,

    nub_rank: Option<u32>,
    nub_points: Option<f64>,
    pro_rank: Option<u32>,
    pro_points: Option<f64>,

    #[schema(value_type = crate::openapi::shims::Timestamp)]
    submitted_at: Timestamp,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetRecordsQuery {
    /// Only include PBs.
    #[serde(default)]
    top: bool,

    /// Only include records set by this player.
    player: Option<PlayerIdentifier>,

    /// Only include records set on this server.
    server: Option<ServerIdentifier>,

    /// Only include records set on this map.
    map: Option<MapIdentifier>,

    /// Only include records set on this course.
    course: Option<String>,

    /// Only include records set on this mode.
    #[param(value_type = Option<crate::openapi::shims::Mode>)]
    mode: Option<Mode>,

    /// Restrict the results to records that (do not) have teleports.
    has_teleports: Option<bool>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<1000, 100>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

/// Returns the latest records.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/records",
    tag = "Records",
    params(GetRecordsQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<Record>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_records(
    State(cx): State<Context>,
    Query(GetRecordsQuery {
        top,
        player,
        server,
        map,
        course,
        mode,
        has_teleports,
        limit,
        offset,
    }): Query<GetRecordsQuery>,
) -> Result<Json<Paginated<Vec<Record>>>, ErrorResponse> {
    let player_id = match player {
        None => None,
        Some(PlayerIdentifier::Id(id)) => Some(id),
        Some(PlayerIdentifier::Name(ref name)) => {
            match cs2kz::players::get_by_name(&cx, name).await {
                Ok(Some(player)) => Some(player.id),
                Ok(None) => return Ok(Json(Paginated::new(0, Vec::new()))),
                Err(error) => return Err(ErrorResponse::internal_server_error(error)),
            }
        },
    };

    let server_id = match server {
        None => None,
        Some(ServerIdentifier::Id(id)) => Some(id),
        Some(ServerIdentifier::Name(ref name)) => {
            match cs2kz::servers::get_by_name(&cx, name).await {
                Ok(Some(server)) => Some(server.id),
                Ok(None) => return Ok(Json(Paginated::new(0, Vec::new()))),
                Err(error) => return Err(ErrorResponse::internal_server_error(error)),
            }
        },
    };

    let (map_id, map) = match map {
        None => (None, None),
        Some(MapIdentifier::Id(map_id)) => (Some(map_id), None),
        Some(MapIdentifier::Name(ref map_name)) => {
            match cs2kz::maps::get_by_name(&cx, map_name).try_next().await {
                Ok(Some(map)) => (Some(map.id), Some(map)),
                Ok(None) => return Ok(Json(Paginated::new(0, Vec::new()))),
                Err(error) => return Err(ErrorResponse::internal_server_error(error)),
            }
        },
    };

    let course_id = match course {
        None => None,
        Some(ref course) => match (map_id, map) {
            (_, Some(ref map)) => match map.find_course_by_name(course).map(|course| course.id) {
                Some(course_id) => Some(course_id),
                None => return Ok(Json(Paginated::new(0, Vec::new()))),
            },
            (Some(map_id), None) => match cs2kz::maps::get_by_id(&cx, map_id).await {
                Ok(Some(map)) => match map.find_course_by_name(course).map(|course| course.id) {
                    Some(course_id) => Some(course_id),
                    None => return Ok(Json(Paginated::new(0, Vec::new()))),
                },
                Ok(None) => return Ok(Json(Paginated::new(0, Vec::new()))),
                Err(error) => return Err(ErrorResponse::internal_server_error(error)),
            },
            (None, None) => match cs2kz::maps::get_course_id_by_name(&cx, course).await {
                Ok(Some(course_id)) => Some(course_id),
                Ok(None) => return Ok(Json(Paginated::new(0, Vec::new()))),
                Err(error) => return Err(ErrorResponse::internal_server_error(error)),
            },
        },
    };

    let params = cs2kz::records::GetRecordsParams {
        top,
        player_id,
        server_id,
        map_id,
        course_id,
        mode,
        has_teleports,
        limit,
        offset,
    };

    let records = cs2kz::records::get(&cx, params)
        .map_ok(Paginated::map_into)
        .and_then(Paginated::collect)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .await?;

    Ok(Json(records))
}

/// Returns the record with the specified ID.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/records/{record_id}",
    tag = "Records",
    params(("record_id" = u32, Path)),
    responses(
        (status = 200, body = Record),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_record(
    State(cx): State<Context>,
    Path(record_id): Path<RecordId>,
) -> Result<Json<Record>, ErrorResponse> {
    let record = cs2kz::records::get_by_id(&cx, record_id)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(record.into()))
}

/// Returns the replay file for a specific record.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/records/{record_id}/replay",
    tag = "Records",
    params(("record_id" = u32, Path)),
    responses(
        (status = 200, body = ReplayFile),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_record_replay(
    State(cx): State<Context>,
    Path(record_id): Path<RecordId>,
) -> Result<ReplayFile, ErrorResponse> {
    let bytes = cs2kz::records::get_replay(&cx, record_id)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(ReplayFile::new(bytes))
}

impl From<cs2kz::records::Record> for Record {
    fn from(record: cs2kz::records::Record) -> Self {
        Self {
            id: record.id,
            player: record.player.into(),
            server: record.server.into(),
            map: record.map.into(),
            course: record.course.into(),
            mode: record.mode,
            styles: record.styles,
            teleports: record.teleports,
            time: record.time,
            nub_rank: record.nub_rank,
            pro_rank: record.pro_rank,
            nub_points: record.nub_points,
            pro_points: record.pro_points,
            submitted_at: record.submitted_at,
        }
    }
}
