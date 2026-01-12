use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::handler::Handler;
use axum::response::NoContent;
use axum::routing::{MethodRouter, Router};
use cs2kz::Context;
use cs2kz::access_keys::AccessKey;
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::servers::{ApproveServerError, ServerHost, ServerId, UpdateServerError};
use cs2kz::time::Timestamp;
use cs2kz::users::{Permission, UserId};
use futures_util::TryFutureExt;

use crate::config::CookieConfig;
use crate::extract::{Json, Path, Query};
use crate::middleware::auth::session_auth;
use crate::middleware::auth::session_auth::Session;
use crate::middleware::auth::session_auth::authorization::{
    AuthorizeSession,
    HasPermissions,
    IsServerOwner,
};
use crate::response::{Created, ErrorResponse};
use crate::users::UserInfo;

mod server_identifier;
pub use server_identifier::ServerIdentifier;

pub fn router<S>(cx: Context, cookie_config: impl Into<Arc<CookieConfig>>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let session_auth_state = session_auth::State::new(cx.clone(), cookie_config)
        .authorize_with(HasPermissions::new(Permission::Servers));
    let is_admin = axum::middleware::from_fn_with_state(session_auth_state.clone(), session_auth);
    let is_admin_or_owner = axum::middleware::from_fn_with_state(
        session_auth_state.map_authorization(|auth| auth.or(IsServerOwner::new(cx))),
        session_auth,
    );

    Router::new()
        .route(
            "/",
            MethodRouter::new()
                .post(approve_server.layer(is_admin.clone()))
                .get(get_servers),
        )
        .route(
            "/{server}",
            MethodRouter::new()
                .patch(update_server.layer(is_admin_or_owner.clone()))
                .get(get_server),
        )
        .route(
            "/{server}/access-key",
            MethodRouter::new()
                .put(refresh_server_access_key.layer(is_admin_or_owner))
                .delete(delete_server_access_key.layer(is_admin)),
        )
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetServersQuery {
    /// Only include servers whose name matches this value.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    #[param(min_length = 1)]
    name: Option<String>,

    /// Only include servers whose host matches this value.
    #[param(value_type = Option<crate::openapi::shims::ServerHost>)]
    host: Option<ServerHost>,

    /// Only include servers owned by this user.
    #[param(value_type = Option<crate::openapi::shims::SteamId64>)]
    owned_by: Option<UserId>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<1000, 250>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Server {
    #[schema(value_type = u16, minimum = 1)]
    id: ServerId,
    name: String,

    /// The server's IP address / domain.
    #[schema(value_type = crate::openapi::shims::ServerHost)]
    host: ServerHost,
    port: u16,

    /// The user who owns this server.
    owner: UserInfo,

    /// When this server was approved by the API.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    approved_at: Timestamp,

    /// A2S query information about the server.
    ///
    /// If this is not available, the server is either offline or came online very recently.
    a2s_info: Option<A2SInfo>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct A2SInfo {
    /// The map the server is currently hosting.
    current_map: String,

    /// The number of players currently playing on the server.
    num_players: u8,

    /// The maximum number of players that can join the server.
    max_players: u8,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ServerInfo {
    /// The server's ID.
    #[schema(value_type = u16, minimum = 1)]
    pub(crate) id: ServerId,

    /// The server's name.
    pub(crate) name: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewServer {
    /// The server's name.
    ///
    /// This has to be a unique value and will be displayed in UIs.
    #[serde(deserialize_with = "crate::serde::deserialize_non_empty")]
    #[schema(min_length = 1)]
    name: String,

    /// The server's IP address / domain.
    #[schema(value_type = crate::openapi::shims::ServerHost)]
    host: ServerHost,

    /// The server's connection port.
    port: u16,

    /// The ID of the user who owns this server.
    #[schema(value_type = crate::openapi::shims::SteamId64)]
    owner_id: UserId,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CreatedServer {
    #[schema(value_type = u16, minimum = 1)]
    server_id: ServerId,

    /// The server's access key.
    #[schema(value_type = crate::openapi::shims::AccessKey)]
    access_key: AccessKey,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CreatedAccessKey {
    /// The server's new access key.
    #[schema(value_type = crate::openapi::shims::AccessKey)]
    access_key: AccessKey,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ServerUpdate {
    /// A new name.
    #[serde(default, deserialize_with = "crate::serde::deserialize_non_empty")]
    name: Option<String>,

    /// A new host IP / domain.
    #[schema(value_type = Option<crate::openapi::shims::ServerHost>)]
    host: Option<ServerHost>,

    /// A new port.
    port: Option<u16>,

    /// A new owner.
    #[schema(value_type = Option<crate::openapi::shims::SteamId64>)]
    owner_id: Option<UserId>,
}

/// Approves a new CS2 server.
///
/// This will generate an access key which allows the server to submit records, jumpstats, etc.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    post,
    path = "/servers",
    tag = "CS2 Servers",
    request_body = NewServer,
    responses(
        (status = 201, body = CreatedServer),
        (status = 401,),
        (status = 409, description = "The server's name or host+port combination is already in use."),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn approve_server(
    State(cx): State<Context>,
    Json(NewServer { name, host, port, owner_id }): Json<NewServer>,
) -> Result<Created<CreatedServer>, ErrorResponse> {
    let server = cs2kz::servers::NewServer { name: &name, host: &host, port, owner_id };

    cs2kz::servers::approve(&cx, server)
        .await
        .map(|(server_id, access_key)| Created(CreatedServer { server_id, access_key }))
        .map_err(|err| match err {
            ApproveServerError::NameAlreadyTaken => ErrorResponse::server_name_already_taken(),
            ApproveServerError::HostAndPortAlreadyTaken => {
                ErrorResponse::server_host_and_port_already_taken()
            },
            ApproveServerError::OwnerDoesNotExist => ErrorResponse::server_owner_does_not_exist(),
            ApproveServerError::Database(error) => ErrorResponse::internal_server_error(error),
        })
}

/// Returns the most recently approved CS2 servers.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/servers",
    tag = "CS2 Servers",
    params(GetServersQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<Server>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_servers(
    State(cx): State<Context>,
    Query(GetServersQuery { name, host, owned_by, limit, offset }): Query<GetServersQuery>,
) -> Result<Json<Paginated<Vec<Server>>>, ErrorResponse> {
    let params = cs2kz::servers::GetServersParams {
        name: name.as_deref(),
        host: host.as_ref(),
        owned_by,
        limit,
        offset,
    };

    let servers = cs2kz::servers::get(&cx, params)
        .map_ok(Paginated::map_into)
        .and_then(Paginated::collect)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .await?;

    Ok(Json(servers))
}

/// Returns the CS2 server with the specified ID / name.
///
/// If you specify a name, it does not have to be an exact match, although exact matches will be
/// preferred.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/servers/{server}",
    tag = "CS2 Servers",
    params(("server" = ServerIdentifier, Path, description = "a server ID or name")),
    responses(
        (status = 200, body = Server),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_server(
    State(cx): State<Context>,
    Path(server_identifier): Path<ServerIdentifier>,
) -> Result<Json<Server>, ErrorResponse> {
    let server = match server_identifier {
        ServerIdentifier::Id(id) => cs2kz::servers::get_by_id(&cx, id).await,
        ServerIdentifier::Name(ref name) => cs2kz::servers::get_by_name(&cx, name).await,
    }
    .map_err(|err| ErrorResponse::internal_server_error(err))?
    .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(server.into()))
}

/// Updates a server's metadata.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    patch,
    path = "/servers/{server_id}",
    tag = "CS2 Servers",
    params(("server_id" = u16, Path, description = "the server's ID")),
    request_body = ServerUpdate,
    responses(
        (status = 204,),
        (status = 401,),
        (status = 404,),
        (status = 409, description = "The specified name or host+port combination is already in use."),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn update_server(
    State(cx): State<Context>,
    Path(server_id): Path<ServerId>,
    Json(ServerUpdate { name, host, port, owner_id }): Json<ServerUpdate>,
) -> Result<NoContent, ErrorResponse> {
    let update = cs2kz::servers::ServerUpdate {
        id: server_id,
        name: name.as_deref(),
        host: host.as_ref(),
        port,
        owner_id,
    };

    match cs2kz::servers::update(&cx, update).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(UpdateServerError::NameAlreadyTaken) => Err(ErrorResponse::server_name_already_taken()),
        Err(UpdateServerError::HostAndPortAlreadyTaken) => {
            Err(ErrorResponse::server_host_and_port_already_taken())
        },
        Err(UpdateServerError::OwnerDoesNotExist) => {
            Err(ErrorResponse::server_owner_does_not_exist())
        },
        Err(UpdateServerError::Database(error)) => Err(ErrorResponse::internal_server_error(error)),
    }
}

/// Generates a new access key for a server and invalidates the old one.
///
/// A successful request to this endpoint will also terminate the server's open WebSocket
/// connection, if any.
#[tracing::instrument(skip(cx, session), fields(session.id = %session.id()), ret(level = "debug"))]
#[utoipa::path(
    put,
    path = "/servers/{server_id}/access-key",
    tag = "CS2 Servers",
    params(("server_id" = u16, Path, description = "the server's ID")),
    responses(
        (status = 201, body = CreatedAccessKey),
        (status = 401,),
        (status = 404,),
    ),
)]
async fn refresh_server_access_key(
    State(cx): State<Context>,
    Path(server_id): Path<ServerId>,
    session: Session,
) -> Result<Created<CreatedAccessKey>, ErrorResponse> {
    let is_admin = session.user().permissions().contains(Permission::Servers);

    if !is_admin {
        match cs2kz::servers::has_access_key(&cx, server_id).await {
            cs2kz::servers::HasAccessKeyResult::DatabaseError(error) => {
                return Err(ErrorResponse::internal_server_error(error));
            },
            cs2kz::servers::HasAccessKeyResult::ServerDoesNotExist => {
                return Err(ErrorResponse::not_found());
            },
            cs2kz::servers::HasAccessKeyResult::HasNoAccessKey => {
                return Err(ErrorResponse::server_owner_cannot_reactivate_server());
            },
            cs2kz::servers::HasAccessKeyResult::HasAccessKey => {},
        }
    }

    let access_key = AccessKey::new();
    let updated = cs2kz::servers::set_access_key(&cx, server_id, Some(access_key))
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?;

    if !updated {
        return Err(ErrorResponse::not_found());
    }

    Ok(Created(CreatedAccessKey { access_key }))
}

/// Deletes a server's access key.
///
/// A successful request to this endpoint will also terminate the server's open WebSocket
/// connection, if any.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    delete,
    path = "/servers/{server_id}/access-key",
    tag = "CS2 Servers",
    params(("server_id" = u16, Path, description = "the server's ID")),
    responses(
        (status = 204,),
        (status = 401,),
        (status = 404,),
    ),
)]
async fn delete_server_access_key(
    State(cx): State<Context>,
    Path(server_id): Path<ServerId>,
) -> Result<NoContent, ErrorResponse> {
    match cs2kz::servers::set_access_key(&cx, server_id, None).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(error) => Err(ErrorResponse::internal_server_error(error)),
    }
}

impl From<cs2kz::servers::Server> for Server {
    fn from(server: cs2kz::servers::Server) -> Self {
        Server {
            id: server.id,
            name: server.name,
            host: server.host,
            port: server.port,
            owner: UserInfo { id: server.owner.id, name: server.owner.name },
            approved_at: server.approved_at,
            a2s_info: cs2kz::steam::servers::with_info(server.id, |info| A2SInfo {
                current_map: info.map.clone(),
                num_players: info.players,
                max_players: info.max_players,
            }),
        }
    }
}

impl From<cs2kz::servers::ServerInfo> for ServerInfo {
    fn from(server: cs2kz::servers::ServerInfo) -> Self {
        Self { id: server.id, name: server.name }
    }
}
