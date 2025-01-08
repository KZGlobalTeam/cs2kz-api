use std::borrow::Cow;
use std::future;
use std::net::Ipv4Addr;

use futures_util::{Stream, StreamExt, TryStreamExt, stream};
use sqlx::QueryBuilder;
use sqlx::types::Json as SqlJson;

use crate::maps::courses::filters::Tier;
use crate::mode::Mode;
use crate::pagination::{Limit, Offset, Paginated};
use crate::time::Timestamp;
use crate::{Context, database};

mod player_id;
pub use player_id::PlayerId;

/// [`cs2kz-metamod`] preferences.
///
/// This is an arbitrary JSON blob set by CS2 servers.
///
/// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Preferences(serde_json::Map<String, serde_json::Value>);

#[derive(Debug)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub ip_address: Option<Ipv4Addr>,
    pub first_joined_at: Timestamp,
    pub last_joined_at: Timestamp,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
}

#[derive(Debug)]
pub struct Profile {
    pub id: PlayerId,
    pub name: String,
    pub rating: f64,
    pub nub_completion: [u32; 8],
    pub pro_completion: [u32; 8],
    pub first_joined_at: Timestamp,
}
#[derive(Debug)]
pub struct GetPlayersParams<'a> {
    pub name: Option<&'a str>,
    pub limit: Limit<1000, 250>,
    pub offset: Offset,
}

#[derive(Debug)]
pub struct NewPlayer<'a> {
    pub id: PlayerId,
    pub name: Cow<'a, str>,
    pub ip_address: Option<Ipv4Addr>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RegisterPlayerInfo {
    pub is_banned: bool,

    #[sqlx(json)]
    pub preferences: Preferences,
}

#[derive(Debug, Display, Error, From)]
pub enum CreatePlayerError {
    #[display("player already exists")]
    PlayerAlreadyExists,

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get players")]
#[from(forward)]
pub struct GetPlayersError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to set player preferences")]
#[from(forward)]
pub struct SetPlayerPreferencesError(database::Error);

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn register(
    cx: &Context,
    NewPlayer { id, name, ip_address }: NewPlayer<'_>,
) -> Result<RegisterPlayerInfo, CreatePlayerError> {
    sqlx::query!(
        "INSERT IGNORE INTO Players (id, name, ip_address)
         VALUES (?, ?, ?)",
        id,
        name,
        ip_address,
    )
    .execute(cx.database().as_ref())
    .await?;

    let is_banned = sqlx::query_scalar!(
        "SELECT (COUNT(*) > 0) AS `is_banned: bool`
         FROM Bans AS b
         RIGHT JOIN Unbans AS ub ON ub.ban_id = b.id
         WHERE b.player_id = ?
         AND (b.id IS NULL OR b.expires_at > NOW())",
        id,
    )
    .fetch_one(cx.database().as_ref())
    .await?;

    let SqlJson(preferences) = sqlx::query_scalar!(
        "SELECT preferences AS `preferences: SqlJson<Preferences>`
         FROM Players
         WHERE id = ?",
        id,
    )
    .fetch_one(cx.database().as_ref())
    .await?;

    Ok(RegisterPlayerInfo { is_banned, preferences })
}

#[tracing::instrument(skip(cx, players), err(level = "debug"))]
pub async fn create_many<'a>(
    cx: &Context,
    players: impl IntoIterator<Item = NewPlayer<'a>>,
) -> Result<(), CreatePlayerError> {
    let mut query = QueryBuilder::new("INSERT IGNORE INTO Players (id, name, ip_address)");

    query.push_values(players, |mut query, NewPlayer { id, name, ip_address }| {
        query.push_bind(id);
        query.push_bind(name);
        query.push_bind(ip_address);
    });

    query
        .build()
        .execute(cx.database().as_ref())
        .await
        .map_err(database::Error::from)
        .map_err(|err| {
            if err.is_unique_violation_of("id") {
                CreatePlayerError::PlayerAlreadyExists
            } else {
                CreatePlayerError::Database(err)
            }
        })?;

    Ok(())
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    GetPlayersParams { name, limit, offset }: GetPlayersParams<'_>,
) -> Result<Paginated<impl Stream<Item = Result<Player, GetPlayersError>>>, GetPlayersError> {
    let total = database::count!(cx.database().as_ref(), "Players").await?;
    let servers = self::macros::select!(
        "WHERE name LIKE COALESCE(?, name)
         LIMIT ?
         OFFSET ?",
        name.map(|name| format!("%{name}%")),
        limit.value(),
        offset.value()
    )
    .fetch(cx.database().as_ref())
    .map_err(GetPlayersError::from);

    Ok(Paginated::new(total, servers))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(
    cx: &Context,
    player_id: PlayerId,
) -> Result<Option<Player>, GetPlayersError> {
    self::macros::select!("WHERE id = ?", player_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetPlayersError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_name(
    cx: &Context,
    player_name: &str,
) -> Result<Option<Player>, GetPlayersError> {
    self::macros::select!("WHERE name LIKE ?", format!("%{player_name}%"))
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetPlayersError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_profile(
    cx: &Context,
    player_id: PlayerId,
    mode: Mode,
) -> Result<Option<Profile>, GetPlayersError> {
    let Some(mut profile) = sqlx::query!(
        r#"WITH RankedPoints AS (
             SELECT
               source,
               record_id,
               ROW_NUMBER() OVER (
                 PARTITION BY player_id
                 ORDER BY points DESC
               ) AS n
             FROM ((
               SELECT "pro" AS source, record_id, player_id, points
               FROM BestProRecords
               WHERE player_id = ?
             ) UNION ALL (
               SELECT "nub" AS source, record_id, player_id, points
               FROM BestNubRecords
               WHERE player_id = ?
             )) AS _
           ),
           NubRecords AS (
             SELECT
               r.id AS record_id,
               r.player_id,
               cf.nub_tier AS tier,
               BestNubRecords.points,
               RANK() OVER (
                 PARTITION BY r.filter_id
                 ORDER BY
                   r.time ASC,
                   r.submitted_at ASC
               ) AS rank
             FROM Records AS r
             JOIN BestNubRecords ON BestNubRecords.record_id = r.id
             JOIN CourseFilters AS cf ON cf.id = r.filter_id
             WHERE r.player_id = ?
             AND cf.mode = ?
           ),
           ProRecords AS (
             SELECT
               r.id AS record_id,
               r.player_id,
               cf.pro_tier AS tier,
               BestProRecords.points,
               RANK() OVER (
                 PARTITION BY r.filter_id
                 ORDER BY
                   r.time ASC,
                   r.submitted_at ASC
               ) AS rank
             FROM Records AS r
             JOIN BestProRecords ON BestProRecords.record_id = r.id
             JOIN CourseFilters AS cf ON cf.id = r.filter_id
             WHERE r.player_id = ?
             AND cf.mode = ?
           ),
           NubRatings AS (
             SELECT
               player_id,
               SUM(KZ_POINTS(tier, false, rank - 1, points) * POWER(0.975, n - 1)) AS rating
             FROM NubRecords
             JOIN RankedPoints
               ON RankedPoints.record_id = NubRecords.record_id
               AND RankedPoints.source = "nub"
             GROUP BY player_id
           ),
           ProRatings AS (
             SELECT
               player_id,
               SUM(KZ_POINTS(tier, true, rank - 1, points) * POWER(0.975, n - 1)) AS rating
             FROM ProRecords
             JOIN RankedPoints
               ON RankedPoints.record_id = ProRecords.record_id
               AND RankedPoints.source = "pro"
             GROUP BY ProRecords.player_id
           )
           SELECT
             p.id AS `player_id: PlayerId`,
             p.name AS player_name,
             NubRatings.rating AS nub_rating,
             ProRatings.rating AS pro_rating,
             p.first_joined_at
           FROM Players AS p
           LEFT JOIN NubRatings ON NubRatings.player_id = p.id
           LEFT JOIN ProRatings ON ProRatings.player_id = p.id
           WHERE p.id = ?"#,
        player_id,
        player_id,
        player_id,
        mode,
        player_id,
        mode,
        player_id,
    )
    .fetch_optional(cx.database().as_ref())
    .await
    .map(|row| {
        row.map(|row| Profile {
            id: row.player_id,
            name: row.player_name,
            rating: match (row.nub_rating, row.pro_rating) {
                (None, Some(_)) => unreachable!(),
                (None, None) => 0.0,
                (Some(nub_rating), None) => nub_rating,
                // ?
                (Some(nub_rating), Some(pro_rating)) => nub_rating + pro_rating,
            },
            nub_completion: [0; 8],
            pro_completion: [0; 8],
            first_joined_at: row.first_joined_at.into(),
        })
    })?
    else {
        return Ok(None);
    };

    let mut pbs = sqlx::query!(
        "SELECT
           r.teleports,
           cf.nub_tier AS `nub_tier: Tier`,
           cf.pro_tier AS `pro_tier: Tier`
         FROM Records AS r
         JOIN CourseFilters AS cf ON cf.id = r.filter_id
         LEFT JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
         LEFT JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id
         WHERE r.player_id = ?
         AND (NOT ((NubRecords.points IS NULL) AND (ProRecords.points IS NULL)))",
        player_id,
    )
    .fetch(cx.database().as_ref());

    while let Some(row) = pbs.try_next().await? {
        profile.nub_completion[(row.nub_tier as usize) - 1] += 1;

        if row.teleports == 0 {
            profile.pro_completion[(row.pro_tier as usize) - 1] += 1;
        }
    }

    Ok(Some(profile))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_preferences(
    cx: &Context,
    player_id: PlayerId,
) -> Result<Option<Preferences>, GetPlayersError> {
    sqlx::query_scalar!(
        "SELECT preferences AS `preferences: SqlJson<Preferences>`
         FROM Players
         WHERE id = ?",
        player_id,
    )
    .fetch_optional(cx.database().as_ref())
    .await
    .map(|row| row.map(|SqlJson(preferences)| preferences))
    .map_err(GetPlayersError::from)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn set_preferences(
    cx: &Context,
    player_id: PlayerId,
    preferences: &Preferences,
) -> Result<bool, SetPlayerPreferencesError> {
    sqlx::query!(
        "UPDATE Players
         SET preferences = ?
         WHERE id = ?",
        SqlJson(preferences),
        player_id,
    )
    .execute(cx.database().as_ref())
    .await
    .map(|result| result.rows_affected() > 0)
    .map_err(SetPlayerPreferencesError::from)
}

#[tracing::instrument(skip(cx, mapper_ids))]
pub fn filter_unknown(
    cx: &Context,
    mapper_ids: impl IntoIterator<Item = PlayerId>,
) -> impl Stream<Item = Result<PlayerId, GetPlayersError>> {
    stream::iter(mapper_ids)
        .then(async |player_id| -> database::Result<(PlayerId, u64)> {
            let count =
                database::count!(cx.database().as_ref(), "Players WHERE id = ?", player_id).await?;

            Ok((player_id, count))
        })
        .map_err(GetPlayersError::from)
        .try_filter(|&(_, count)| future::ready(count > 0))
        .map_ok(|(player_id, _)| player_id)
}

mod macros {
    macro_rules! select {
        ( $($extra:tt)* ) => {
            sqlx::query_as!(
                Player,
                "SELECT
                   id AS `id: PlayerId`,
                   name,
                   ip_address AS `ip_address: Ipv4Addr`,
                   first_joined_at,
                   last_joined_at
                 FROM Players "
                + $($extra)*
            )
        };
    }

    pub(super) use select;
}
