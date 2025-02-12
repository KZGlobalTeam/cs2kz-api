use std::net::Ipv4Addr;
use std::num::NonZero;

use futures_util::{Stream, TryStreamExt};
use sqlx::Row;

use crate::pagination::{Limit, Offset, Paginated};
use crate::players::{PlayerId, PlayerInfo};
use crate::time::Timestamp;
use crate::users::UserId;
use crate::{Context, database};

mod banned_by;
pub use banned_by::BannedBy;

mod reason;
pub use reason::BanReason;

define_id_type! {
    /// A unique identifier for player bans.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct BanId(NonZero<u32>);
}

#[derive(Debug)]
pub struct Ban {
    pub id: BanId,
    pub player: PlayerInfo,
    pub banned_by: BannedBy,
    pub reason: BanReason,
    pub unban: Option<Unban>,
    pub created_at: Timestamp,
}

#[derive(Debug)]
pub struct Unban {
    pub admin_id: UserId,
    pub reason: String,
    pub created_at: Timestamp,
}

#[derive(Debug)]
pub struct GetBansParams {
    pub player_id: Option<PlayerId>,
    pub banned_by: Option<UserId>,
    pub reason: Option<BanReason>,
    pub limit: Limit<1000, 100>,
    pub offset: Offset,
}

#[derive(Debug)]
pub struct NewBan {
    pub player_id: PlayerId,
    pub player_ip: Option<Ipv4Addr>,
    pub banned_by: BannedBy,
    pub reason: BanReason,
}

#[derive(Debug)]
pub struct BanUpdate {
    pub id: BanId,
    pub reason: Option<BanReason>,
    pub expires_at: Option<Timestamp>,
}

#[derive(Debug)]
pub struct NewUnban {
    pub ban_id: BanId,
    pub admin_id: UserId,
    pub reason: String,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get bans")]
#[from(forward)]
pub struct GetBansError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to create ban")]
pub enum CreateBanError {
    #[display("player is already banned")]
    AlreadyBanned,

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to revert ban")]
#[from(forward)]
pub struct CreateUnbanError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to update ban")]
#[from(forward)]
pub struct UpdateBanError(database::Error);

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    GetBansParams { player_id, banned_by, reason, limit, offset }: GetBansParams,
) -> Result<Paginated<impl Stream<Item = Result<Ban, GetBansError>>>, GetBansError> {
    let total = database::count!(cx.database().as_ref(), "Bans").await?;
    let bans = self::macros::select!(
        "WHERE b.player_id = COALESCE(?, b.player_id)
         AND b.banned_by = COALESCE(?, b.banned_by)
         AND b.reason = COALESCE(?, b.reason)
         LIMIT ?
         OFFSET ?",
        player_id,
        banned_by,
        reason,
        limit.value(),
        offset.value(),
    )
    .fetch(cx.database().as_ref())
    .map_ok(|row| self::macros::parse_row!(row))
    .map_err(GetBansError::from);

    Ok(Paginated::new(total, bans))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(cx: &Context, ban_id: BanId) -> Result<Option<Ban>, GetBansError> {
    self::macros::select!("WHERE b.id = ?", ban_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map(|row| row.map(|row| self::macros::parse_row!(row)))
        .map_err(GetBansError::from)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn create(
    cx: &Context,
    NewBan { player_id, player_ip, banned_by, reason }: NewBan,
) -> Result<BanId, CreateBanError> {
    cx.database_transaction(async move |conn| {
        let active_ban_count = database::count!(
            &mut *conn,
            "Bans AS b
             LEFT JOIN Unbans AS ub ON ub.ban_id = b.id
             WHERE b.player_id = ?
             AND (
               ub.ban_id IS NULL
               OR b.expires_at > NOW()
             )",
            player_id,
        )
        .await?;

        if active_ban_count > 0 {
            return Err(CreateBanError::AlreadyBanned);
        }

        sqlx::query!(
            "INSERT INTO Bans (player_id, player_ip, banned_by, reason, plugin_version_id)
             VALUES (
               ?,
               COALESCE(?, (SELECT ip_address FROM Players WHERE id = ?)),
               ?,
               ?,
               (SELECT id FROM PluginVersions ORDER BY published_at DESC LIMIT 1)
             )",
            player_id,
            player_ip,
            player_id,
            banned_by,
            reason,
        )
        .fetch_one(conn)
        .await
        .and_then(|row| row.try_get(0))
        .map_err(CreateBanError::from)
    })
    .await
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn update(
    cx: &Context,
    BanUpdate { id, reason, expires_at }: BanUpdate,
) -> Result<bool, UpdateBanError> {
    sqlx::query!(
        "UPDATE Bans
         SET reason = COALESCE(?, reason),
             expires_at = COALESCE(?, expires_at)
         WHERE id = ?",
        reason,
        expires_at,
        id,
    )
    .execute(cx.database().as_ref())
    .await
    .map(|result| result.rows_affected() > 0)
    .map_err(UpdateBanError::from)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn revert(
    cx: &Context,
    NewUnban { ban_id, admin_id, reason }: NewUnban,
) -> Result<bool, CreateUnbanError> {
    cx.database_transaction(async move |conn| {
        let updated = sqlx::query!("UPDATE Bans SET expires_at = NOW() WHERE id = ?", ban_id)
            .execute(&mut *conn)
            .await
            .map(|result| result.rows_affected() > 0)?;

        if !updated {
            return Ok(false);
        }

        sqlx::query!(
            "INSERT INTO Unbans (ban_id, admin_id, reason)
             VALUES (?, ?, ?)",
            ban_id,
            admin_id,
            reason
        )
        .execute(conn)
        .await?;

        Ok(true)
    })
    .await
}

mod macros {
    macro_rules! select {
        ( $($extra:tt)* ) => {
            sqlx::query!(
                "SELECT
                   b.id AS `id: BanId`,
                   p.id AS `player_id: PlayerId`,
                   p.name AS player_name,
                   b.banned_by AS `banned_by: BannedBy`,
                   b.reason AS `reason: BanReason`,
                   ub.admin_id AS `unban_admin_id: UserId`,
                   ub.reason AS unban_reason,
                   ub.created_at AS unban_created_at,
                   b.created_at
                 FROM Bans AS b
                 JOIN Players AS p ON p.id = b.player_id
                 LEFT JOIN Unbans AS ub ON ub.ban_id = b.id "
                + $($extra)*
            )
        };
    }

    macro_rules! parse_row {
        ($row:expr) => {
            Ban {
                id: $row.id,
                player: PlayerInfo { id: $row.player_id, name: $row.player_name },
                banned_by: $row.banned_by,
                reason: $row.reason,
                unban: try {
                    let admin_id = $row.unban_admin_id?;
                    let reason = $row
                        .unban_reason
                        .expect("if unban admin_id existed, unban reason must also exist");
                    let created_at = $row
                        .unban_created_at
                        .map(Timestamp::from)
                        .expect("if unban admin_id existed, unban created_at must also exist");

                    Unban { admin_id, reason, created_at }
                },
                created_at: $row.created_at.into(),
            }
        };
    }

    pub(super) use {parse_row, select};
}
