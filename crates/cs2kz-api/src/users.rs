use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::response::NoContent;
use axum::routing::{self, MethodRouter, Router};
use cs2kz::Context;
use cs2kz::email::Email;
use cs2kz::time::Timestamp;
use cs2kz::users::{Permission, Permissions, UserId};
use futures_util::TryStreamExt;

use crate::config::CookieConfig;
use crate::extract::{Json, Path, Query};
use crate::middleware::auth::session_auth;
use crate::middleware::auth::session_auth::authorization::HasPermissions;
use crate::response::ErrorResponse;

pub fn router<S>(cx: Context, cookie_config: impl Into<Arc<CookieConfig>>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let session_auth_state = session_auth::State::new(cx, cookie_config);
    let is_logged_in =
        axum::middleware::from_fn_with_state(session_auth_state.clone(), session_auth);
    let is_admin = axum::middleware::from_fn_with_state(
        session_auth_state.authorize_with(HasPermissions::new(Permission::UserPermissions)),
        session_auth,
    );

    Router::new()
        .route("/", routing::get(get_users))
        .route("/{user_id}", routing::get(get_user))
        .route(
            "/{user_id}/email",
            MethodRouter::new()
                .put(update_user_email)
                .delete(delete_user_email)
                .layer(is_logged_in),
        )
        .route("/{user_id}/permissions", routing::put(update_user_permissions).layer(is_admin))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetUsersQuery {
    /// Only include users with these permissions.
    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Permissions)]
    permissions: Permissions,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct User {
    /// The user's SteamID.
    #[schema(value_type = crate::openapi::shims::SteamId64)]
    id: UserId,

    /// The user's last-known name on Steam.
    #[schema(example = "AlphaKeks")]
    name: String,

    /// The user's API permissions.
    #[schema(value_type = crate::openapi::shims::Permissions)]
    permissions: Permissions,

    /// When this user was registered to the API.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    registered_at: Timestamp,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UserInfo {
    /// The user's SteamID.
    #[schema(value_type = crate::openapi::shims::SteamId64)]
    pub(crate) id: UserId,

    /// The user's last-known name on Steam.
    pub(crate) name: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateUserEmailPayload {
    /// The new email address.
    #[schema(value_type = str, format = Email)]
    email: Email,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateUserPermissionsPayload {
    /// The new permissions.
    #[schema(value_type = crate::openapi::shims::Permissions)]
    permissions: Permissions,
}

/// Returns all users with permissions.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/users",
    tag = "Users",
    params(GetUsersQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<User>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_users(
    State(cx): State<Context>,
    Query(GetUsersQuery { permissions }): Query<GetUsersQuery>,
) -> Result<Json<Vec<User>>, ErrorResponse> {
    let users = cs2kz::users::get(&cx, cs2kz::users::GetUsersParams { permissions })
        .map_ok(User::from)
        .map_err(|err| ErrorResponse::internal_server_error(err))
        .try_collect()
        .await?;

    Ok(Json(users))
}

/// Returns the user with the specified ID.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/users/{user_id}",
    tag = "Users",
    params(("user_id" = crate::openapi::shims::SteamId64, Path, description = "the user's SteamID")),
    responses(
        (status = 200, body = User),
        (status = 400, description = "invalid path parameters"),
        (status = 404,),
    ),
)]
async fn get_user(
    State(cx): State<Context>,
    Path(user_id): Path<UserId>,
) -> Result<Json<User>, ErrorResponse> {
    let user = cs2kz::users::get_by_id(&cx, user_id)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(user.into()))
}

/// Updates a user's email address.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    put,
    path = "/users/{user_id}/email",
    tag = "Users",
    params(("user_id" = crate::openapi::shims::SteamId64, Path, description = "the user's SteamID")),
    request_body = UpdateUserEmailPayload,
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 404,),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn update_user_email(
    State(cx): State<Context>,
    Path(user_id): Path<UserId>,
    Json(UpdateUserEmailPayload { email }): Json<UpdateUserEmailPayload>,
) -> Result<NoContent, ErrorResponse> {
    update_user(&cx, cs2kz::users::UserUpdate {
        user_id,
        email: Some(cs2kz::users::EmailUpdate::Update(&email)),
        permissions: None,
        mark_as_seen: false,
    })
    .await
}

/// Deletes a user's email address.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    delete,
    path = "/users/{user_id}/email",
    tag = "Users",
    params(("user_id" = crate::openapi::shims::SteamId64, Path, description = "the user's SteamID")),
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 404,),
    ),
)]
async fn delete_user_email(
    State(cx): State<Context>,
    Path(user_id): Path<UserId>,
) -> Result<NoContent, ErrorResponse> {
    update_user(&cx, cs2kz::users::UserUpdate {
        user_id,
        email: Some(cs2kz::users::EmailUpdate::Clear),
        permissions: None,
        mark_as_seen: false,
    })
    .await
}

/// Update a user's permissions.
///
/// This will **replace their current permissions**!
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    put,
    path = "/users/{user_id}/permissions",
    tag = "Users",
    params(("user_id" = crate::openapi::shims::SteamId64, Path, description = "the user's SteamID")),
    request_body = UpdateUserPermissionsPayload,
    responses(
        (status = 204,),
        (status = 400, description = "invalid path parameters"),
        (status = 401,),
        (status = 404,),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn update_user_permissions(
    State(cx): State<Context>,
    Path(user_id): Path<UserId>,
    Json(UpdateUserPermissionsPayload { permissions }): Json<UpdateUserPermissionsPayload>,
) -> Result<NoContent, ErrorResponse> {
    update_user(&cx, cs2kz::users::UserUpdate {
        user_id,
        email: None,
        permissions: Some(permissions),
        mark_as_seen: false,
    })
    .await
}

#[tracing::instrument(skip(cx))]
async fn update_user(
    cx: &Context,
    update: cs2kz::users::UserUpdate<'_>,
) -> Result<NoContent, ErrorResponse> {
    match cs2kz::users::update(cx, update).await {
        Ok(true) => Ok(NoContent),
        Ok(false) => Err(ErrorResponse::not_found()),
        Err(error) => Err(ErrorResponse::internal_server_error(error)),
    }
}

impl From<cs2kz::users::User> for User {
    fn from(user: cs2kz::users::User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            permissions: user.permissions,
            registered_at: user.registered_at,
        }
    }
}
