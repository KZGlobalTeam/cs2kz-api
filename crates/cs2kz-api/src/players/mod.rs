use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::handler::Handler;
use axum::response::NoContent;
use axum::routing::{self, MethodRouter, Router};
use cs2kz::Context;
use cs2kz::mode::Mode;
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::players::{PlayerId, Preferences};
use cs2kz::time::Timestamp;
use futures_util::TryFutureExt;

use crate::config::{CookieConfig, SteamAuthConfig};
use crate::extract::{Json, Path, Query};
use crate::middleware::auth::session_auth;
use crate::middleware::auth::session_auth::authorization::IsPlayer;
use crate::response::ErrorResponse;
use crate::steam::{self, SteamUser};

mod player_identifier;
pub use player_identifier::PlayerIdentifier;

#[derive(Clone)]
struct GetSteamProfileState {
    http_client: reqwest::Client,
    auth_config: Arc<SteamAuthConfig>,
}

pub fn router<S>(
    cx: Context,
    auth_config: impl Into<Arc<SteamAuthConfig>>,
    cookie_config: impl Into<Arc<CookieConfig>>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let session_auth_state =
        session_auth::State::new(cx.clone(), cookie_config).authorize_with(IsPlayer::new());
    let is_player = axum::middleware::from_fn_with_state(session_auth_state, session_auth);

    Router::new()
        .route("/", routing::get(get_players))
        .route("/{player}", routing::get(get_player))
        .route("/{player}/profile", routing::get(get_player_profile))
        .route(
            "/{player}/steam-profile",
            routing::get(get_player_steam_profile).with_state(GetSteamProfileState {
                http_client: reqwest::Client::new(),
                auth_config: auth_config.into(),
            }),
        )
        .route(
            "/{player}/preferences",
            MethodRouter::new()
                .put(update_player_preferences.layer(is_player))
                .get(get_player_preferences),
        )
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetPlayersQuery {
    /// Only include players whose name matches this value.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    name: Option<String>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<1000, 250>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetPlayerProfileQuery {
    #[param(value_type = crate::openapi::shims::Mode)]
    mode: Mode,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Player {
    /// The player's SteamID.
    #[schema(value_type = crate::openapi::shims::SteamId)]
    id: PlayerId,

    /// The player's name on Steam.
    name: String,

    /// When this player first joined an approved CS2 server.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    first_joined_at: Timestamp,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PlayerProfile {
    /// The player's SteamID.
    #[schema(value_type = crate::openapi::shims::SteamId)]
    id: PlayerId,

    /// The player's name on Steam.
    name: String,

    rating: f64,
    nub_completion: [u32; 8],
    pro_completion: [u32; 8],

    /// When this player first joined an approved CS2 server.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    first_joined_at: Timestamp,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PlayerInfo {
    /// The player's SteamID.
    #[schema(value_type = crate::openapi::shims::SteamId)]
    pub(crate) id: PlayerId,

    /// The player's name on Steam.
    pub(crate) name: String,
}

/// Returns CS2 players.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/players",
    tag = "Players",
    params(GetPlayersQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<Player>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_players(
    State(cx): State<Context>,
    Query(GetPlayersQuery { name, limit, offset }): Query<GetPlayersQuery>,
) -> Result<Json<Paginated<Vec<Player>>>, ErrorResponse> {
    let params = cs2kz::players::GetPlayersParams { name: name.as_deref(), limit, offset };
    let players = cs2kz::players::get(&cx, params)
        .map_ok(Paginated::map_into)
        .and_then(Paginated::collect)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .await?;

    Ok(Json(players))
}

/// Returns the player with the specified ID / name.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/players/{player}",
    tag = "Players",
    params(("player" = PlayerIdentifier, Path, description = "a SteamID or name")),
    responses(
        (status = 200, body = Player),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_player(
    State(cx): State<Context>,
    Path(player_identifier): Path<PlayerIdentifier>,
) -> Result<Json<Player>, ErrorResponse> {
    let player = match player_identifier {
        PlayerIdentifier::Id(id) => cs2kz::players::get_by_id(&cx, id).await,
        PlayerIdentifier::Name(ref name) => cs2kz::players::get_by_name(&cx, name).await,
    }
    .map_err(|err| ErrorResponse::internal_server_error(err))?
    .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(player.into()))
}

/// Returns a player's profile information.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/players/{player_id}/profile",
    tag = "Players",
    params(
        ("player_id" = u64, Path, description = "the player's SteamID"),
        GetPlayerProfileQuery,
    ),
    responses(
        (status = 200, body = Player),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_player_profile(
    State(cx): State<Context>,
    Path(player_id): Path<PlayerId>,
    Query(GetPlayerProfileQuery { mode }): Query<GetPlayerProfileQuery>,
) -> Result<Json<PlayerProfile>, ErrorResponse> {
    let profile = cs2kz::players::get_profile(&cx, player_id, mode)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(profile.into()))
}

/// Returns a player's Steam profile.
#[tracing::instrument(skip(http_client))]
#[utoipa::path(
    get,
    path = "/players/{player_id}/steam-profile",
    tag = "Players",
    params(("player_id" = crate::openapi::shims::SteamId, Path, description = "the player's SteamID")),
    responses(
        (status = 200, body = SteamUser),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
        (status = 502, description = "Steam returned an error"),
    ),
)]
async fn get_player_steam_profile(
    State(GetSteamProfileState { http_client, auth_config }): State<GetSteamProfileState>,
    Path(player_id): Path<PlayerId>,
) -> Result<Json<SteamUser>, ErrorResponse> {
    let user = steam::fetch_user(&http_client, &auth_config.web_api_key, player_id.into())
        .await?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(user))
}

/// Returns a player's cs2kz-metamod preferences.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/players/{player_id}/preferences",
    tag = "Players",
    params(("player_id" = u64, Path, description = "the player's SteamID")),
    responses(
        (status = 200, body = Object),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_player_preferences(
    State(cx): State<Context>,
    Path(player_id): Path<PlayerId>,
) -> Result<Json<Preferences>, ErrorResponse> {
    cs2kz::players::get_preferences(&cx, player_id)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .map(Json)
        .ok_or_else(ErrorResponse::not_found)
}

/// Replaces a player's cs2kz-metamod preferences.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    put,
    path = "/players/{player_id}/preferences",
    tag = "Players",
    params(("player_id" = u64, Path, description = "the player's SteamID")),
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 404,),
    ),
)]
async fn update_player_preferences(
    State(cx): State<Context>,
    Path(player_id): Path<PlayerId>,
    Json(preferences): Json<Preferences>,
) -> Result<NoContent, ErrorResponse> {
    match cs2kz::players::set_preferences(&cx, player_id, &preferences).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(error) => Err(ErrorResponse::internal_server_error(error)),
    }
}

impl From<cs2kz::players::Player> for Player {
    fn from(player: cs2kz::players::Player) -> Self {
        Self {
            id: player.id,
            name: player.name,
            first_joined_at: player.first_joined_at,
        }
    }
}

impl From<cs2kz::players::PlayerInfo> for PlayerInfo {
    fn from(player: cs2kz::players::PlayerInfo) -> Self {
        Self { id: player.id, name: player.name }
    }
}

impl From<cs2kz::players::Profile> for PlayerProfile {
    fn from(profile: cs2kz::players::Profile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            rating: profile.rating,
            nub_completion: profile.nub_completion,
            pro_completion: profile.pro_completion,
            first_joined_at: profile.first_joined_at,
        }
    }
}
