use std::num::{NonZero, TryFromIntError};

use futures_util::{Stream, TryStreamExt};
use sqlx::Row;

use crate::access_keys::AccessKey;
use crate::pagination::{Limit, Offset, Paginated};
use crate::time::Timestamp;
use crate::users::{UserId, UserInfo};
use crate::{Context, database};

mod server_host;
pub use server_host::ServerHost;

define_id_type! {
    /// A unique identifier for CS2 servers.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct ServerId(NonZero<u16>);

    impl TryFrom<u64> for ServerId {
        type Error = TryFromIntError;

        fn try_from(value: u64) -> Result<Self, Self::Error> {
            u16::try_from(value)?.try_into().map(Self)
        }
    }
}

#[derive(Debug)]
pub struct Server {
    pub id: ServerId,
    pub name: String,
    pub host: ServerHost,
    pub port: u16,
    pub owner: UserInfo,
    pub access_key: Option<AccessKey>,
    pub approved_at: Timestamp,
    pub last_connected_at: Option<Timestamp>,
}

#[derive(Debug, serde::Serialize)]
pub struct ServerInfo {
    pub id: ServerId,
    pub name: String,
}

#[derive(Debug)]
pub struct GetServersParams<'a> {
    pub name: Option<&'a str>,
    pub host: Option<&'a ServerHost>,
    pub owned_by: Option<UserId>,
    pub limit: Limit<1000, 250>,
    pub offset: Offset,
}

#[derive(Debug)]
pub struct NewServer<'a> {
    pub name: &'a str,
    pub host: &'a ServerHost,
    pub port: u16,
    pub owner_id: UserId,
}

#[derive(Debug)]
pub struct ServerUpdate<'a> {
    pub id: ServerId,
    pub name: Option<&'a str>,
    pub host: Option<&'a ServerHost>,
    pub port: Option<u16>,
    pub owner_id: Option<UserId>,
}

#[derive(Debug, Display, Error, From)]
pub enum ApproveServerError {
    #[display("name already in use")]
    NameAlreadyTaken,

    #[display("host+port combination already in use")]
    HostAndPortAlreadyTaken,

    #[display("there is no user with that ID")]
    OwnerDoesNotExist,

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
pub enum UpdateServerError {
    #[display("name already in use")]
    NameAlreadyTaken,

    #[display("host+port combination already in use")]
    HostAndPortAlreadyTaken,

    #[display("there is no user with that ID")]
    OwnerDoesNotExist,

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, From)]
pub enum HasAccessKeyResult {
    ServerDoesNotExist,
    HasAccessKey,
    HasNoAccessKey,

    #[from(forward)]
    DatabaseError(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get servers")]
#[from(forward)]
pub struct GetServersError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to update server access key")]
#[from(forward)]
pub struct UpdateAccessKeyError(database::Error);

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    GetServersParams { name, host, owned_by, limit, offset }: GetServersParams<'_>,
) -> Result<Paginated<impl Stream<Item = Result<Server, GetServersError>>>, GetServersError> {
    let total = database::count!(cx.database().as_ref(), "Servers").await?;
    let servers = self::macros::select!(
        "WHERE s.name LIKE COALESCE(?, s.name)
         AND s.host = COALESCE(?, s.host)
         AND s.owner_id = COALESCE(?, s.owner_id)
         LIMIT ?
         OFFSET ?",
        name.map(|name| format!("%{name}%")),
        host,
        owned_by,
        limit.value(),
        offset.value()
    )
    .fetch(cx.database().as_ref())
    .map_ok(|row| self::macros::parse_row!(row))
    .map_err(GetServersError::from);

    Ok(Paginated::new(total, servers))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(
    cx: &Context,
    server_id: ServerId,
) -> Result<Option<Server>, GetServersError> {
    self::macros::select!("WHERE s.id = ?", server_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map(|row| row.map(|row| self::macros::parse_row!(row)))
        .map_err(GetServersError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_access_key(
    cx: &Context,
    access_key: &AccessKey,
) -> Result<Option<Server>, GetServersError> {
    self::macros::select!("WHERE s.access_key = ?", access_key)
        .fetch_optional(cx.database().as_ref())
        .await
        .map(|row| row.map(|row| self::macros::parse_row!(row)))
        .map_err(GetServersError::from)
}

#[tracing::instrument(skip(cx), ret(level = "debug"))]
pub async fn has_access_key(cx: &Context, server_id: ServerId) -> HasAccessKeyResult {
    match sqlx::query_scalar!("SELECT access_key FROM Servers WHERE id = ?", server_id)
        .fetch_optional(cx.database().as_ref())
        .await
    {
        Ok(None) => HasAccessKeyResult::ServerDoesNotExist,
        Ok(Some(None)) => HasAccessKeyResult::HasNoAccessKey,
        Ok(Some(Some(_))) => HasAccessKeyResult::HasAccessKey,
        Err(error) => HasAccessKeyResult::DatabaseError(error.into()),
    }
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_name(
    cx: &Context,
    server_name: &str,
) -> Result<Option<Server>, GetServersError> {
    self::macros::select!("WHERE s.name LIKE ?", format!("%{server_name}%"))
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetServersError::from)
        .map(|row| row.map(|row| self::macros::parse_row!(row)))
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn approve(
    cx: &Context,
    NewServer { name, host, port, owner_id }: NewServer<'_>,
) -> Result<(ServerId, AccessKey), ApproveServerError> {
    let access_key = AccessKey::new();
    let server_id = sqlx::query!(
        "INSERT INTO Servers (name, host, port, owner_id, access_key)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id",
        name,
        host,
        port,
        owner_id,
        access_key,
    )
    .fetch_one(cx.database().as_ref())
    .await
    .and_then(|row| row.try_get(0))
    .map_err(database::Error::from)
    .map_err(|err| {
        if err.is_unique_violation_of("name") {
            ApproveServerError::NameAlreadyTaken
        } else if err.is_unique_violation_of("UC_host_port") {
            ApproveServerError::HostAndPortAlreadyTaken
        } else if err.is_fk_violation_of("owner_id") {
            ApproveServerError::OwnerDoesNotExist
        } else {
            ApproveServerError::Database(err)
        }
    })?;

    Ok((server_id, access_key))
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn set_access_key(
    cx: &Context,
    server_id: ServerId,
    access_key: Option<AccessKey>,
) -> Result<bool, UpdateAccessKeyError> {
    sqlx::query!("UPDATE Servers SET access_key = ? WHERE id = ?", access_key, server_id)
        .execute(cx.database().as_ref())
        .await
        .map(|result| result.rows_affected() > 0)
        .map_err(|err| UpdateAccessKeyError(err.into()))
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn update(
    cx: &Context,
    ServerUpdate { id, name, host, port, owner_id }: ServerUpdate<'_>,
) -> Result<bool, UpdateServerError> {
    sqlx::query!(
        "UPDATE Servers
         SET name = COALESCE(?, name),
             host = COALESCE(?, host),
             port = COALESCE(?, port),
             owner_id = COALESCE(?, owner_id)
         WHERE id = ?",
        name,
        host,
        port,
        owner_id,
        id,
    )
    .execute(cx.database().as_ref())
    .await
    .map(|result| result.rows_affected() > 0)
    .map_err(database::Error::from)
    .map_err(|err| {
        if err.is_unique_violation_of("name") {
            UpdateServerError::NameAlreadyTaken
        } else if err.is_unique_violation_of("UC_host_port") {
            UpdateServerError::HostAndPortAlreadyTaken
        } else if err.is_fk_violation_of("owner_id") {
            UpdateServerError::OwnerDoesNotExist
        } else {
            UpdateServerError::Database(err)
        }
    })
}

mod macros {
    macro_rules! select {
        ( $($extra:tt)* ) => {
            sqlx::query!(
                "SELECT
                   s.id AS `id: ServerId`,
                   s.name,
                   s.host AS `host: ServerHost`,
                   s.port,
                   o.id AS `owner_id: UserId`,
                   o.name AS owner_name,
                   s.access_key AS `access_key: AccessKey`,
                   s.approved_at,
                   s.last_connected_at
                 FROM Servers AS s
                 JOIN Users AS o ON o.id = s.owner_id "
                + $($extra)*
            )
        };
    }

    macro_rules! parse_row {
        ($row:expr) => {
            Server {
                id: $row.id,
                name: $row.name,
                host: $row.host,
                port: $row.port,
                owner: UserInfo { id: $row.owner_id, name: $row.owner_name },
                access_key: $row.access_key,
                approved_at: $row.approved_at.into(),
                last_connected_at: $row.last_connected_at.map(Into::into),
            }
        };
    }

    pub(super) use {parse_row, select};
}
