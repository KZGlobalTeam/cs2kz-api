use futures_util::{Stream, TryStreamExt};

use crate::email::Email;
use crate::time::Timestamp;
use crate::{Context, database};

mod user_id;
pub use user_id::UserId;

pub mod permissions;
pub use permissions::{Permission, Permissions};

pub mod sessions;

#[derive(Debug)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub permissions: Permissions,
    pub registered_at: Timestamp,
}

#[derive(Debug)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
}

#[derive(Debug)]
pub struct GetUsersParams {
    pub permissions: Permissions,
}

#[derive(Debug)]
pub struct UserUpdate<'a> {
    pub user_id: UserId,
    pub email: Option<EmailUpdate<'a>>,
    pub permissions: Option<Permissions>,
    pub mark_as_seen: bool,
}

#[derive(Debug)]
pub enum EmailUpdate<'a> {
    Clear,
    Update(&'a Email),
}

#[derive(Debug)]
pub struct NewUser<'a> {
    pub id: UserId,
    pub name: &'a str,
}

#[derive(Debug, Display, Error, From)]
pub enum CreateUserError {
    #[display("user already exists")]
    UserAlreadyExists,

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get users")]
#[from(forward)]
pub struct GetUsersError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to update user")]
#[from(forward)]
pub struct UpdateUserError(database::Error);

#[tracing::instrument(skip(cx))]
pub fn get(
    cx: &Context,
    GetUsersParams { permissions }: GetUsersParams,
) -> impl Stream<Item = Result<User, GetUsersError>> {
    sqlx::query_as!(
        User,
        "SELECT
           id AS `id: UserId`,
           name,
           permissions AS `permissions: Permissions`,
           registered_at
         FROM Users
         WHERE permissions > 0
         AND (permissions & ?) = ?",
        permissions,
        permissions,
    )
    .fetch(cx.database().as_ref())
    .map_err(Into::into)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(cx: &Context, user_id: UserId) -> Result<Option<User>, GetUsersError> {
    sqlx::query_as!(
        User,
        "SELECT
           id AS `id: UserId`,
           name,
           permissions AS `permissions: Permissions`,
           registered_at
         FROM Users
         WHERE id = ?",
        user_id,
    )
    .fetch_optional(cx.database().as_ref())
    .await
    .map_err(Into::into)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn update(
    cx: &Context,
    UserUpdate { user_id, email, permissions, mark_as_seen }: UserUpdate<'_>,
) -> Result<bool, UpdateUserError> {
    let last_login_at = mark_as_seen.then(Timestamp::now);

    match email {
        Some(EmailUpdate::Clear) => sqlx::query!(
            "UPDATE Users
             SET email_address = NULL,
                 permissions = COALESCE(?, permissions),
                 last_login_at = COALESCE(?, last_login_at)
             WHERE id = ?",
            permissions,
            last_login_at,
            user_id,
        )
        .execute(cx.database().as_ref()),
        email_update @ (None | Some(EmailUpdate::Update(_))) => sqlx::query!(
            "UPDATE Users
             SET email_address = COALESCE(?, email_address),
                 permissions = COALESCE(?, permissions),
                 last_login_at = COALESCE(?, last_login_at)
             WHERE id = ?",
            match email_update {
                None => None,
                Some(EmailUpdate::Update(email)) => Some(email),
                Some(EmailUpdate::Clear) => unreachable!(),
            },
            permissions,
            last_login_at,
            user_id,
        )
        .execute(cx.database().as_ref()),
    }
    .await
    .map(|result| result.rows_affected() > 0)
    .map_err(Into::into)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn create(
    cx: &Context,
    NewUser { id, name }: NewUser<'_>,
) -> Result<(), CreateUserError> {
    sqlx::query!("INSERT INTO Users (id, name) VALUES (?, ?)", id, name)
        .execute(cx.database().as_ref())
        .await
        .map_err(database::Error::from)
        .map_err(|err| {
            if err.is_unique_violation_of("id") {
                CreateUserError::UserAlreadyExists
            } else {
                CreateUserError::Database(err)
            }
        })?;

    Ok(())
}
