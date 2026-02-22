use axum::extract::{FromRef, State};
use axum::routing::{self, Router};
use cs2kz::Context;
use cs2kz::jumpstats::{JumpType, JumpstatId};
use cs2kz::mode::Mode;
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::styles::Styles;
use cs2kz::time::Seconds;
use futures_util::TryFutureExt;

use crate::extract::{Json, Path, Query};
use crate::players::PlayerInfo;
use crate::response::ErrorResponse;
use crate::servers::ServerInfo;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    Router::new()
        .route("/", routing::get(get_jumpstats))
        .route("/{jumpstat_id}", routing::get(get_jumpstat))
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Jumpstat {
    #[schema(value_type = u32, minimum = 1)]
    id: JumpstatId,
    player: PlayerInfo,
    server: ServerInfo,

    #[schema(value_type = crate::openapi::shims::Mode)]
    mode: Mode,

    #[schema(value_type = crate::openapi::shims::Styles)]
    styles: Styles,

    #[schema(value_type = crate::openapi::shims::JumpType)]
    jump_type: JumpType,

    /// Airtime in seconds.
    #[schema(value_type = f64)]
    time: Seconds,
    strafes: u8,
    distance: f32,
    sync: f32,
    pre: f32,
    max: f32,
    overlap: f32,
    bad_angles: f32,
    dead_air: f32,
    height: f32,
    airpath: f32,
    deviation: f32,
    average_width: f32,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetJumpstatsQuery {
    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<1000, 100>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

/// Returns the latest jumpstats.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/jumpstats",
    tag = "Jumpstats",
    params(GetJumpstatsQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<Jumpstat>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_jumpstats(
    State(cx): State<Context>,
    Query(GetJumpstatsQuery { limit, offset }): Query<GetJumpstatsQuery>,
) -> Result<Json<Paginated<Vec<Jumpstat>>>, ErrorResponse> {
    let params = cs2kz::jumpstats::GetJumpstatsParams { limit, offset };
    let jumpstats = cs2kz::jumpstats::get(&cx, params)
        .map_ok(Paginated::map_into)
        .and_then(Paginated::collect)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .await?;

    Ok(Json(jumpstats))
}

/// Returns the jumpstat with the specified ID.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/jumpstats/{jumpstat_id}",
    tag = "Jumpstats",
    params(("jumpstat_id" = u32, Path)),
    responses(
        (status = 200, body = Jumpstat),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_jumpstat(
    State(cx): State<Context>,
    Path(jumpstat_id): Path<JumpstatId>,
) -> Result<Json<Jumpstat>, ErrorResponse> {
    let jumpstat = cs2kz::jumpstats::get_by_id(&cx, jumpstat_id)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(jumpstat.into()))
}

impl From<cs2kz::jumpstats::Jumpstat> for Jumpstat {
    fn from(jump: cs2kz::jumpstats::Jumpstat) -> Self {
        Self {
            id: jump.id,
            player: jump.player.into(),
            server: jump.server.into(),
            mode: jump.mode,
            styles: jump.styles,
            jump_type: jump.jump_type,
            time: jump.time,
            strafes: jump.strafes,
            distance: jump.distance,
            sync: jump.sync,
            pre: jump.pre,
            max: jump.max,
            overlap: jump.overlap,
            bad_angles: jump.bad_angles,
            dead_air: jump.dead_air,
            height: jump.height,
            airpath: jump.airpath,
            deviation: jump.deviation,
            average_width: jump.average_width,
        }
    }
}
