use std::borrow::Cow;
use std::future;
use std::net::Ipv4Addr;

use futures_util::{Stream, StreamExt, TryFutureExt as _, TryStreamExt, stream};
use sqlx::types::Json as SqlJson;

use crate::Context;
use crate::database::{self, QueryBuilder};
use crate::mode::Mode;
use crate::pagination::{Limit, Offset, Paginated};
use crate::time::Timestamp;

mod player_id;
pub use player_id::PlayerId;

/// [`cs2kz-metamod`] preferences.
///
/// This is an arbitrary JSON blob set by CS2 servers.
///
/// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Preferences(serde_json::Map<String, serde_json::Value>);

#[derive(Debug, sqlx::FromRow)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub vnl_rating: f64,
    pub ckz_rating: f64,
    pub is_banned: bool,
    pub first_joined_at: Timestamp,
    pub last_joined_at: Timestamp,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct PlayerInfoWithIsBanned {
    pub id: PlayerId,
    pub name: String,
    pub is_banned: bool,
}

#[derive(Debug, Default)]
pub struct GetPlayersParams<'a> {
    pub name: Option<&'a str>,
    pub sort_by: SortBy,
    pub limit: Limit<1000, 250>,
    pub offset: Offset,
}

#[derive(Debug, Default, serde::Deserialize)]
pub enum SortBy {
    /// Sort by most recent players.
    #[default]
    #[serde(rename = "join-date")]
    JoinDate,

    /// Sort by VNL rating.
    #[serde(rename = "vnl-rating")]
    RatingVNL,

    /// Sort by CKZ rating.
    #[serde(rename = "ckz-rating")]
    RatingCKZ,
}

#[derive(Debug)]
#[cfg_attr(feature = "fake", derive(fake::Dummy))]
pub struct NewPlayer<'a> {
    pub id: PlayerId,
    #[cfg_attr(
        feature = "fake",
        dummy(expr = "Cow::Owned(fake::Fake::fake(&fake::faker::internet::en::Username()))")
    )]
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
#[display("failed to calculate rating")]
#[from(forward)]
pub struct CalculateRatingError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to update ratings")]
#[from(forward)]
pub struct UpdateRatingsError(database::Error);

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
        "INSERT INTO Players (id, name, ip_address)
         VALUES (?, ?, ?)
         ON DUPLICATE KEY
         UPDATE name = VALUES(name),
                ip_address = VALUES(ip_address)",
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
    GetPlayersParams { name, sort_by, limit, offset }: GetPlayersParams<'_>,
) -> Result<Paginated<impl Stream<Item = Result<Player, GetPlayersError>>>, GetPlayersError> {
    let total = database::count!(
        cx.database().as_ref(),
        "Players WHERE name LIKE COALESCE(?, name)",
        name.map(|name| format!("%{name}%")),
    )
    .await?;

    let mut query = QueryBuilder::new(
        "WITH BanCounts AS (
           SELECT b.player_id, COUNT(*) AS count
            FROM Bans AS b
            RIGHT JOIN Unbans AS ub ON ub.ban_id = b.id
            WHERE (b.id IS NULL OR b.expires_at > NOW())
         )
         SELECT
           p.id,
           p.name,
           p.vnl_rating,
           p.ckz_rating,
           (COALESCE(BanCounts.count, 0) > 0) AS is_banned,
           p.first_joined_at,
           p.last_joined_at
         FROM Players AS p
         LEFT JOIN BanCounts ON BanCounts.player_id = p.id
         WHERE p.name LIKE COALESCE(?, p.name)",
    );

    query.push(" ORDER BY ");
    query.push(match sort_by {
        SortBy::JoinDate => " p.first_joined_at DESC ",
        SortBy::RatingVNL => " p.vnl_rating DESC ",
        SortBy::RatingCKZ => " p.ckz_rating DESC ",
    });
    query.push(" LIMIT ? OFFSET ? ");

    let players = query
        .build_query_as::<Player>()
        .bind(name.map(|name| format!("%{name}%")))
        .bind(limit.value())
        .bind(offset.value())
        .fetch(cx.database().as_ref())
        .map_err(GetPlayersError::from)
        .try_collect::<Vec<_>>()
        .map_ok(stream::iter)
        .await?;

    Ok(Paginated::new(total, players.map(Ok)))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(
    cx: &Context,
    player_id: PlayerId,
) -> Result<Option<Player>, GetPlayersError> {
    self::macros::select!("WHERE p.id = ?", player_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetPlayersError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_name(
    cx: &Context,
    player_name: &str,
) -> Result<Option<Player>, GetPlayersError> {
    self::macros::select!("WHERE p.name LIKE ?", format!("%{player_name}%"))
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetPlayersError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn calculate_rating(
    cx: &Context,
    player_id: PlayerId,
    mode: Mode,
) -> Result<f64, CalculateRatingError> {
    let rating = sqlx::query_scalar!(
        "with RelevantRecords as (
           select
             row_number() over (
               partition by filter_id, is_pro_leaderboard
               order by time asc, record_id asc, is_pro_leaderboard asc
             ) as leaderboard_rank,
             points,
             player_id,
             record_id,
             tier,
             is_pro_leaderboard
           from
             ((
               select
                 br.*,
                 cf.nub_tier as tier,
                 false as is_pro_leaderboard
               from
                 BestNubRecords as br
                 inner join CourseFilters as cf on cf.id = br.filter_id
               where
                 cf.state = 1
                 and cf.mode = ?
             ) union all (
               select
                 br.*,
                 cf.pro_tier as tier,
                 true as is_pro_leaderboard
               from
                 BestProRecords as br
                 inner join CourseFilters as cf on cf.id = br.filter_id
               where
                 cf.state = 1
                 and cf.mode = ?
             )) as _
         ),
         RanksWithPoints as (
           select
             rr.*,
             KZ_POINTS(
               rr.tier,
               rr.is_pro_leaderboard,
               rr.leaderboard_rank - 1,
               rr.points
             ) as points2
           from RelevantRecords as rr
         ),
         RankedRecords as (
           select
             row_number() over (
               partition BY rr.player_id
               order by rr.points2 desc, rr.record_id asc, rr.is_pro_leaderboard asc
             ) as overall_rank,
             rr.*
           from RanksWithPoints as rr
         )
         select
           sum(points2 * power(0.975, overall_rank - 1) * 0.1) as rating
         from (select * from RankedRecords where player_id = ?) as _",
        mode,
        mode,
        player_id,
    )
    .fetch_one(cx.database().as_ref())
    .await?;

    Ok(rating.unwrap_or(0.0))
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn update_ratings(cx: &Context, mode: Mode) -> Result<(), UpdateRatingsError> {
    let query = format!(
        "insert into Players (id, name, vnl_rating, ckz_rating)
         select
           player_id,
           '',
           rating,
           rating
         from (
           with RelevantRecords as (
             select
               row_number() over (
                 partition by filter_id, is_pro_leaderboard
                 order by time asc, record_id asc, is_pro_leaderboard asc
               ) as leaderboard_rank,
               points,
               player_id,
               record_id,
               tier,
               is_pro_leaderboard
             from
               ((
                 select
                   br.*,
                   cf.nub_tier as tier,
                   false as is_pro_leaderboard
                 from
                   BestNubRecords as br
                   inner join CourseFilters as cf on cf.id = br.filter_id
                 where cf.state = 1
                 and cf.mode = ?
               ) union all (
                 select
                   br.*,
                   cf.pro_tier as tier,
                   true as is_pro_leaderboard
                 from
                   BestProRecords as br
                   inner join CourseFilters as cf on cf.id = br.filter_id
                 where cf.state = 1
                 and cf.mode = ?
               )) as _
           ),
           RanksWithPoints as (
             select
               rr.*,
               KZ_POINTS(
                 rr.tier,
                 rr.is_pro_leaderboard,
                 rr.leaderboard_rank - 1,
                 rr.points
               ) as points2
             from RelevantRecords as rr
           ),
           RankedRecords as (
             select
               row_number() over (
                 partition BY rr.player_id
                 order by rr.points2 desc, rr.record_id asc, rr.is_pro_leaderboard asc
               ) as overall_rank,
               rr.*
             from RanksWithPoints as rr
           )
           select
             player_id,
             sum(points2 * power(0.975, overall_rank - 1) * 0.1) as rating
           from RankedRecords
           group by player_id
         ) as _
         on duplicate key update {col_to_update}=values({col_to_update})",
        col_to_update = match mode {
            Mode::Vanilla => "vnl_rating",
            Mode::Classic => "ckz_rating",
        },
    );

    sqlx::query(&query)
        .bind(mode)
        .bind(mode)
        .execute(cx.database().as_ref())
        .await?;

    Ok(())
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

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn on_leave(
    cx: &Context,
    player_id: PlayerId,
    name: &str,
    preferences: &Preferences,
) -> Result<bool, SetPlayerPreferencesError> {
    sqlx::query!(
        "UPDATE Players
         SET name = ?,
             preferences = ?
         WHERE id = ?",
        name,
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

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn delete(cx: &Context, count: usize) -> database::Result<u64> {
    sqlx::query!("DELETE FROM Players LIMIT ?", count as u64)
        .execute(cx.database().as_ref())
        .await
        .map(|result| result.rows_affected())
        .map_err(database::Error::from)
}

mod macros {
    macro_rules! select {
        ( $($extra:tt)* ) => {
            sqlx::query_as!(
                Player,
                "WITH BanCounts AS (
                   SELECT b.player_id, COUNT(*) AS count
                    FROM Bans AS b
                    RIGHT JOIN Unbans AS ub ON ub.ban_id = b.id
                    WHERE (b.id IS NULL OR b.expires_at > NOW())
                 )
                 SELECT
                   p.id AS `id: PlayerId`,
                   p.name,
                   p.vnl_rating,
                   p.ckz_rating,
                   (COALESCE(BanCounts.count, 0) > 0) AS `is_banned!: bool`,
                   p.first_joined_at,
                   p.last_joined_at
                 FROM Players AS p
                 LEFT JOIN BanCounts ON BanCounts.player_id = p.id "
                + $($extra)*
            )
        };
    }

    pub(super) use select;
}
