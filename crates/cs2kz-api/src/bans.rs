use std::net::Ipv4Addr;
use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::handler::Handler;
use axum::response::NoContent;
use axum::routing::{MethodRouter, Router};
use cs2kz::Context;
use cs2kz::bans::{BanId, BanReason, BannedBy, CreateBanError};
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::players::PlayerId;
use cs2kz::time::Timestamp;
use cs2kz::users::{Permission, UserId};
use futures_util::TryFutureExt;

use crate::config::CookieConfig;
use crate::extract::{Json, Path, Query};
use crate::middleware::auth::session_auth;
use crate::middleware::auth::session_auth::Session;
use crate::middleware::auth::session_auth::authorization::HasPermissions;
use crate::players::PlayerInfo;
use crate::response::{Created, ErrorResponse};

pub fn router<S>(cx: Context, cookie_config: impl Into<Arc<CookieConfig>>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let session_auth_state = session_auth::State::new(cx.clone(), cookie_config)
        .authorize_with(HasPermissions::new(Permission::PlayerBans));

    let is_admin = axum::middleware::from_fn_with_state(session_auth_state, session_auth);

    Router::new()
        .route(
            "/",
            MethodRouter::new()
                .post(create_ban.layer(is_admin.clone()))
                .get(get_bans),
        )
        .route(
            "/{ban_id}",
            MethodRouter::new()
                .patch(update_ban.layer(is_admin.clone()))
                .delete(delete_ban.layer(is_admin.clone()))
                .get(get_ban),
        )
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetBansQuery {
    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<1000, 100>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Ban {
    #[schema(value_type = u32, minimum = 1)]
    id: BanId,

    player: PlayerInfo,

    #[schema(value_type = crate::openapi::shims::BannedBy)]
    banned_by: BannedBy,

    #[schema(value_type = crate::openapi::shims::BanReason)]
    reason: BanReason,

    /// The unban corresponding to this ban, if any.
    unban: Option<Unban>,

    #[schema(value_type = crate::openapi::shims::Timestamp)]
    created_at: Timestamp,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Unban {
    #[schema(value_type = crate::openapi::shims::SteamId64)]
    admin_id: UserId,

    reason: String,

    #[schema(value_type = crate::openapi::shims::Timestamp)]
    created_at: Timestamp,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewBan {
    /// The player that should be banned.
    #[schema(value_type = crate::openapi::shims::SteamId)]
    player_id: PlayerId,

    /// The player's IP address.
    ///
    /// If left unspecified, the player's last known IP address will be used instead.
    #[schema(value_type = Option<str>, format = Ipv4)]
    player_ip: Option<Ipv4Addr>,

    /// The reason for the ban.
    #[schema(value_type = crate::openapi::shims::BanReason)]
    reason: BanReason,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewUnban {
    /// The reason for the unban.
    #[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
    reason: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct BanUpdate {
    #[schema(value_type = Option<crate::openapi::shims::BanReason>)]
    reason: Option<BanReason>,

    #[serde(deserialize_with = "crate::serde::deserialize_future_timestamp_opt")]
    #[schema(value_type = Option<crate::openapi::shims::Timestamp>)]
    expires_at: Option<Timestamp>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CreatedBan {
    #[schema(value_type = u32, minimum = 1)]
    ban_id: BanId,
}

/// Bans a player.
#[tracing::instrument(skip(cx, session), fields(
    session.id = %session.id(),
    session.user.id = %session.user().id(),
    session.user.permissions = ?session.user().permissions(),
))]
#[utoipa::path(
    post,
    path = "/bans",
    tag = "Player Bans",
    request_body = NewBan,
    responses(
        (status = 201, body = CreatedBan),
        (status = 401,),
        (status = 409, description = "the player is already banned"),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn create_ban(
    State(cx): State<Context>,
    session: Session,
    Json(NewBan { player_id, player_ip, reason }): Json<NewBan>,
) -> Result<Created<CreatedBan>, ErrorResponse> {
    let ban = cs2kz::bans::NewBan {
        player_id,
        player_ip,
        banned_by: BannedBy::Admin(session.user().id()),
        reason,
    };

    cs2kz::bans::create(&cx, ban)
        .await
        .map(|ban_id| Created(CreatedBan { ban_id }))
        .map_err(|err| match err {
            CreateBanError::AlreadyBanned => ErrorResponse::player_already_banned(),
            CreateBanError::Database(error) => ErrorResponse::internal_server_error(error),
        })
}

/// Returns the latest player bans.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/bans",
    tag = "Player Bans",
    params(GetBansQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<Ban>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_bans(
    State(cx): State<Context>,
    Query(GetBansQuery { limit, offset }): Query<GetBansQuery>,
) -> Result<Json<Paginated<Vec<Ban>>>, ErrorResponse> {
    let params = cs2kz::bans::GetBansParams { limit, offset };
    let bans = cs2kz::bans::get(&cx, params)
        .map_ok(Paginated::map_into)
        .and_then(Paginated::collect)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .await?;

    Ok(Json(bans))
}

/// Returns the ban with the specified ID.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/bans/{ban_id}",
    tag = "Player Bans",
    params(("ban_id" = u32, Path)),
    responses(
        (status = 200, body = Ban),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_ban(
    State(cx): State<Context>,
    Path(ban_id): Path<BanId>,
) -> Result<Json<Ban>, ErrorResponse> {
    let ban = cs2kz::bans::get_by_id(&cx, ban_id)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(ban.into()))
}

/// Updates an active ban.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    patch,
    path = "/bans/{ban_id}",
    tag = "Player Bans",
    params(("ban_id" = u32, Path)),
    request_body = BanUpdate,
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 404,),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn update_ban(
    State(cx): State<Context>,
    Path(ban_id): Path<BanId>,
    Json(BanUpdate { reason, expires_at }): Json<BanUpdate>,
) -> Result<NoContent, ErrorResponse> {
    let update = cs2kz::bans::BanUpdate { id: ban_id, reason, expires_at };

    match cs2kz::bans::update(&cx, update).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(error) => Err(ErrorResponse::internal_server_error(error)),
    }
}

/// Reverts a ban and creates an unban.
#[tracing::instrument(skip(cx, session), fields(
    session.id = %session.id(),
    session.user.id = %session.user().id(),
    session.user.permissions = ?session.user().permissions(),
))]
#[utoipa::path(
    delete,
    path = "/bans/{ban_id}",
    tag = "Player Bans",
    params(("ban_id" = u32, Path)),
    request_body = NewUnban,
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 404,),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn delete_ban(
    State(cx): State<Context>,
    session: Session,
    Path(ban_id): Path<BanId>,
    Json(NewUnban { reason }): Json<NewUnban>,
) -> Result<NoContent, ErrorResponse> {
    let unban = cs2kz::bans::NewUnban { ban_id, admin_id: session.user().id(), reason };

    match cs2kz::bans::revert(&cx, unban).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(error) => Err(ErrorResponse::internal_server_error(error)),
    }
}

impl From<cs2kz::bans::Ban> for Ban {
    fn from(ban: cs2kz::bans::Ban) -> Self {
        Self {
            id: ban.id,
            player: ban.player.into(),
            banned_by: ban.banned_by,
            reason: ban.reason,
            unban: ban.unban.map(Into::into),
            created_at: ban.created_at,
        }
    }
}

impl From<cs2kz::bans::Unban> for Unban {
    fn from(unban: cs2kz::bans::Unban) -> Self {
        Self {
            admin_id: unban.admin_id,
            reason: unban.reason,
            created_at: unban.created_at,
        }
    }
}
