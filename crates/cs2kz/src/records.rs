use std::num::NonZero;

use futures_util::{Stream, TryStreamExt};
use sqlx::{QueryBuilder, Row};

use crate::events::{self, Event};
use crate::maps::courses::filters::Tier;
use crate::maps::{CourseFilterId, CourseId, CourseInfo, MapId, MapInfo};
use crate::mode::Mode;
use crate::num::AsF64;
use crate::pagination::{Limit, Offset, Paginated};
use crate::players::{PlayerId, PlayerInfo};
use crate::plugin::PluginVersionId;
use crate::points::{self, CalculatePointsError, Distribution};
use crate::servers::{ServerId, ServerInfo};
use crate::styles::Styles;
use crate::time::{Seconds, Timestamp};
use crate::{Context, database};

define_id_type! {
    /// A unique identifier for records.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct RecordId(NonZero<u32>);
}

#[derive(Debug, serde::Serialize)]
pub struct Record {
    pub id: RecordId,
    pub player: PlayerInfo,
    pub server: ServerInfo,
    pub map: MapInfo,
    pub course: CourseInfo,
    pub mode: Mode,
    pub styles: Styles,
    pub teleports: u32,
    pub time: Seconds,
    pub nub_rank: Option<u32>,
    pub nub_points: Option<f64>,
    pub pro_rank: Option<u32>,
    pub pro_points: Option<f64>,
    pub submitted_at: Timestamp,
}

#[derive(Debug, Clone, Copy)]
pub struct LeaderboardEntry {
    pub record_id: RecordId,
    pub player_id: PlayerId,
    pub teleports: u32,
    pub time: Seconds,
}

#[derive(Debug, Clone, Copy)]
pub struct BestRecord {
    pub id: RecordId,
    pub player_id: PlayerId,
    pub nub_points: f64,
    pub pro_points: ProPoints,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ProPoints {
    pub value: f64,
    pub based_on_pro_leaderboard: bool,
}

impl AsF64 for LeaderboardEntry {
    fn as_f64(&self) -> f64 {
        self.time.as_f64()
    }
}

#[derive(Debug)]
pub struct NewRecord {
    pub player_id: PlayerId,
    pub server_id: ServerId,
    pub filter_id: CourseFilterId,
    pub styles: Styles,
    pub teleports: u32,
    pub time: Seconds,
    pub plugin_version_id: PluginVersionId,
}

#[derive(Debug, Default)]
pub struct GetRecordsParams {
    /// Only include PBs.
    pub top: bool,

    /// Only include records set by this player.
    pub player_id: Option<PlayerId>,

    /// Only include records set on this server.
    pub server_id: Option<ServerId>,

    /// Only include records set on this map.
    pub map_id: Option<MapId>,

    /// Only include records set on this course.
    pub course_id: Option<CourseId>,

    /// Only include records set on this mode.
    pub mode: Option<Mode>,

    /// Restrict the results to records that (do not) have teleports.
    pub has_teleports: Option<bool>,

    pub limit: Limit<1000, 100>,
    pub offset: Offset,
}

#[derive(Debug)]
pub struct SubmittedRecord {
    pub record_id: RecordId,
    pub player_rating: f64,
    pub is_first_nub_record: bool,
    pub nub_rank: Option<u32>,
    pub nub_points: Option<f64>,
    pub nub_leaderboard_size: u32,
    pub is_first_pro_record: bool,
    pub pro_rank: Option<u32>,
    pub pro_points: Option<f64>,
    pub pro_leaderboard_size: u32,
}

#[derive(Debug, Display, Error, From)]
pub enum SubmitRecordError {
    #[display("{_0}")]
    CalculatePoints(CalculatePointsError),

    #[display("{_0}")]
    #[from(forward)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get records")]
#[from(forward)]
pub struct GetRecordsError(database::Error);

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn submit(
    cx: &Context,
    NewRecord {
        player_id,
        server_id,
        filter_id,
        styles,
        teleports,
        time,
        plugin_version_id,
    }: NewRecord,
) -> Result<SubmittedRecord, SubmitRecordError> {
    let record = cx
        .database_transaction(async move |conn| {
            let record_id = sqlx::query!(
                "INSERT INTO Records (
                   player_id,
                   server_id,
                   filter_id,
                   styles,
                   teleports,
                   time,
                   plugin_version_id
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 RETURNING id",
                player_id,
                server_id,
                filter_id,
                styles,
                teleports,
                time,
                plugin_version_id,
            )
            .fetch_one(&mut *conn)
            .await
            .and_then(|row| row.try_get(0))?;

            let old_nub = sqlx::query!(
                "SELECT
                   r.id,
                   r.teleports,
                   r.time,
                   NubRecords.points
                 FROM Records AS r
                 JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
                 WHERE r.filter_id = ?
                 AND r.player_id = ?",
                filter_id,
                player_id,
            )
            .fetch_optional(&mut *conn)
            .await?;

            let old_pro = sqlx::query!(
                "SELECT
                   r.id,
                   r.teleports,
                   r.time,
                   ProRecords.points
                 FROM Records AS r
                 JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id
                 WHERE r.filter_id = ?
                 AND r.player_id = ?",
                filter_id,
                player_id,
            )
            .fetch_optional(&mut *conn)
            .await?;

            let insert_nub =
                async |conn: &mut database::Connection| -> Result<_, SubmitRecordError> {
                    let dist = sqlx::query_as!(
                        Distribution,
                        "SELECT a, b, loc, scale, top_scale
                         FROM PointDistributionData
                         WHERE filter_id = ?
                         AND (NOT is_pro_leaderboard)",
                        filter_id,
                    )
                    .fetch_optional(&mut *conn)
                    .await?;

                    let tier = sqlx::query_scalar!(
                        "SELECT nub_tier AS `tier: Tier`
                         FROM CourseFilters
                         WHERE id = ?",
                        filter_id,
                    )
                    .fetch_one(&mut *conn)
                    .await?;

                    let (leaderboard_size, top_time) = sqlx::query!(
                        "SELECT
                           COUNT(r.id) AS size,
                           MIN(r.time) AS top_time
                         FROM Records AS r
                         JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
                         WHERE r.filter_id = ?
                         GROUP BY r.filter_id",
                        filter_id,
                    )
                    .fetch_optional(&mut *conn)
                    .await
                    .map(|row| row.map_or((0, None), |row| (row.size as usize, row.top_time)))?;

                    let dist_points =
                        if let Some(top_time) = top_time.filter(|&top_time| top_time < time) {
                            points::calculate(dist, tier, leaderboard_size, top_time, time.into())
                                .await
                                .map_err(SubmitRecordError::CalculatePoints)?
                        } else {
                            1.0
                        };

                    sqlx::query!(
                        "INSERT INTO BestNubRecords (
                           filter_id,
                           player_id,
                           record_id,
                           points
                         )
                         VALUES (?, ?, ?, ?)
                         ON DUPLICATE KEY
                         UPDATE record_id = VALUES(record_id),
                                points = VALUES(points)",
                        filter_id,
                        player_id,
                        record_id,
                        dbg!(dist_points),
                    )
                    .execute(&mut *conn)
                    .await?;

                    if leaderboard_size <= points::SMALL_LEADERBOARD_THRESHOLD {
                        let mut top_time = None;
                        let leaderboard = sqlx::query!(
                            "SELECT
                               r.player_id,
                               r.id,
                               r.time
                             FROM Records AS r
                             JOIN BestNubRecords ON BestNubRecords.record_id = r.id
                             WHERE BestNubRecords.filter_id = ?
                             ORDER BY time ASC",
                             filter_id,
                            )
                            .fetch(&mut *conn)
                            .map_ok(|row| {
                                (row.player_id, row.id, points::for_small_leaderboard(
                                    tier,
                                    *top_time.get_or_insert(row.time),
                                    row.time,
                                ))
                            })
                            .try_collect::<Vec<_>>()
                            .await?;

                        let mut query = QueryBuilder::new(
                            "INSERT INTO BestNubRecords (
                               filter_id,
                               player_id,
                               record_id,
                               points
                             )",
                        );

                        query.push_values(leaderboard, |mut query, (player_id, record_id, points)| {
                            query.push_bind(filter_id);
                            query.push_bind(player_id);
                            query.push_bind(record_id);
                            query.push_bind(dbg!(points));
                        });

                        query.push("ON DUPLICATE KEY UPDATE points = VALUES(points)");

                        query
                            .build()
                            .persistent(false)
                            .execute(&mut *conn)
                            .await?;
                    }

                    Ok(move |rank| points::complete(tier, false, rank, dist_points))
                };

            let insert_pro =
                async |conn: &mut database::Connection| -> Result<_, SubmitRecordError> {
                    let dist = sqlx::query_as!(
                        Distribution,
                        "SELECT a, b, loc, scale, top_scale
                         FROM PointDistributionData
                         WHERE filter_id = ?
                         AND (is_pro_leaderboard)",
                        filter_id,
                    )
                    .fetch_optional(&mut *conn)
                    .await?;

                    let tier = sqlx::query_scalar!(
                        "SELECT pro_tier AS `tier: Tier`
                         FROM CourseFilters
                         WHERE id = ?",
                        filter_id,
                    )
                    .fetch_one(&mut *conn)
                    .await?;

                    let (leaderboard_size, top_time) = sqlx::query!(
                        "SELECT
                           COUNT(r.id) AS size,
                           MIN(r.time) AS top_time
                         FROM Records AS r
                         JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id
                         WHERE r.filter_id = ?
                         GROUP BY r.filter_id",
                        filter_id,
                    )
                    .fetch_optional(&mut *conn)
                    .await
                    .map(|row| row.map_or((0, None), |row| (row.size as usize, row.top_time)))?;

                    let dist_points =
                        if let Some(top_time) = top_time.filter(|&top_time| top_time < time) {
                            points::calculate(dist, tier, leaderboard_size, top_time, time.into())
                                .await
                                .map_err(SubmitRecordError::CalculatePoints)?
                        } else {
                            1.0
                        };

                    sqlx::query!(
                        "INSERT INTO BestProRecords (
                           filter_id,
                           player_id,
                           record_id,
                           points,
                           points_based_on_pro_leaderboard
                         )
                         VALUES (?, ?, ?, ?, true)
                         ON DUPLICATE KEY
                         UPDATE record_id = VALUES(record_id),
                                points = VALUES(points)",
                        filter_id,
                        player_id,
                        record_id,
                        dbg!(dist_points),
                    )
                    .execute(&mut *conn)
                    .await?;

                    if leaderboard_size <= points::SMALL_LEADERBOARD_THRESHOLD {
                        let mut top_time = None;
                        let leaderboard = sqlx::query!(
                            "SELECT
                               r.player_id,
                               r.id,
                               r.time
                             FROM Records AS r
                             JOIN BestProRecords ON BestProRecords.record_id = r.id
                             WHERE BestProRecords.filter_id = ?
                             ORDER BY time ASC",
                             filter_id,
                            )
                            .fetch(&mut *conn)
                            .map_ok(|row| {
                                (row.player_id, row.id, points::for_small_leaderboard(
                                    tier,
                                    *top_time.get_or_insert(row.time),
                                    row.time,
                                ))
                            })
                            .try_collect::<Vec<_>>()
                            .await?;

                        let mut query = QueryBuilder::new(
                            "INSERT INTO BestProRecords (
                               filter_id,
                               player_id,
                               record_id,
                               points,
                               points_based_on_pro_leaderboard
                             )",
                        );

                        query.push_values(leaderboard, |mut query, (player_id, record_id, points)| {
                            query.push_bind(filter_id);
                            query.push_bind(player_id);
                            query.push_bind(record_id);
                            query.push_bind(dbg!(points));
                            query.push_bind(true);
                        });

                        query.push("ON DUPLICATE KEY UPDATE points = VALUES(points)");

                        query
                            .build()
                            .persistent(false)
                            .execute(&mut *conn)
                            .await?;
                    }

                    Ok(move |rank| points::complete(tier, true, rank, dist_points))
                };

            let (calc_nub_points, calc_pro_points) = match (&old_nub, &old_pro, teleports) {
                (None, None, 0) => {
                    let calc_nub_points = insert_nub(&mut *conn).await?;
                    let calc_pro_points = insert_pro(&mut *conn).await?;

                    (Some(calc_nub_points), Some(calc_pro_points))
                },
                (None, None, 1..) => {
                    let calc_nub_points = insert_nub(&mut *conn).await?;

                    (Some(calc_nub_points), None)
                },
                (Some(nub), None, 0) => {
                    let calc_nub_points = if time < nub.time {
                        Some(insert_nub(&mut *conn).await?)
                    } else {
                        None
                    };

                    (calc_nub_points, Some(insert_pro(&mut *conn).await?))
                },
                (Some(nub), _, 1..) => {
                    let calc_nub_points = if time < nub.time {
                        Some(insert_nub(&mut *conn).await?)
                    } else {
                        None
                    };

                    (calc_nub_points, None)
                },
                (Some(nub), Some(pro), 0) => {
                    let calc_nub_points = if time < nub.time {
                        Some(insert_nub(&mut *conn).await?)
                    } else {
                        None
                    };

                    let calc_pro_points = if time < pro.time {
                        Some(insert_pro(&mut *conn).await?)
                    } else {
                        None
                    };

                    (calc_nub_points, calc_pro_points)
                },
                (None, Some(pro), 0) => {
                    if time < pro.time {
                        let calc_nub_points = insert_nub(&mut *conn).await?;
                        let calc_pro_points = insert_pro(&mut *conn).await?;

                        (Some(calc_nub_points), Some(calc_pro_points))
                    } else {
                        (None, None)
                    }
                },
                (None, Some(pro), 1..) => {
                    let calc_nub_points = if time < pro.time {
                        Some(insert_nub(&mut *conn).await?)
                    } else {
                        None
                    };

                    (calc_nub_points, None)
                },
            };

            let mode = sqlx::query_scalar!(
                "SELECT mode AS `mode: Mode`
                 FROM CourseFilters
                 WHERE id = ?",
                filter_id,
            )
            .fetch_one(&mut *conn)
            .await?;

            sqlx::query!(
                r#"WITH RankedPoints AS (
                     SELECT
                       source,
                       record_id,
                       ROW_NUMBER() OVER (
                         PARTITION BY player_id
                         ORDER BY points DESC
                       ) AS n
                     FROM ((
                       SELECT "nub" AS source, record_id, player_id, points
                       FROM BestNubRecords
                       WHERE player_id = ?
                     ) UNION ALL (
                       SELECT "pro" AS source, record_id, player_id, points
                       FROM BestProRecords
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
                   NubRankAndPoints AS (
                     SELECT
                       player_id,
                       rank,
                       SUM(KZ_POINTS(tier, false, rank - 1, points)) AS points
                     FROM NubRecords
                     WHERE record_id = ?
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
                     GROUP BY player_id
                   ),
                   ProRankAndPoints AS (
                     SELECT
                       player_id,
                       rank,
                       SUM(KZ_POINTS(tier, false, rank - 1, points)) AS points
                     FROM ProRecords
                     WHERE record_id = ?
                     GROUP BY player_id
                   )
                   SELECT
                     (SELECT COUNT(*) FROM BestNubRecords WHERE filter_id = ?) AS nub_leaderboard_size,
                     (SELECT COUNT(*) FROM BestProRecords WHERE filter_id = ?) AS pro_leaderboard_size,
                     NubRatings.rating AS nub_rating,
                     NubRankAndPoints.rank AS nub_rank,
                     NubRankAndPoints.points AS nub_points,
                     ProRatings.rating AS pro_rating,
                     ProRankAndPoints.rank AS pro_rank,
                     ProRankAndPoints.points AS pro_points
                   FROM Players AS p
                   LEFT JOIN NubRecords ON NubRecords.player_id = p.id
                   LEFT JOIN ProRecords ON ProRecords.player_id = p.id
                   LEFT JOIN NubRatings ON NubRatings.player_id = p.id
                   LEFT JOIN NubRankAndPoints ON NubRankAndPoints.player_id = p.id
                   LEFT JOIN ProRatings ON ProRatings.player_id = p.id
                   LEFT JOIN ProRankAndPoints ON ProRankAndPoints.player_id = p.id
                   WHERE p.id = ?"#,
                player_id,
                player_id,
                player_id,
                mode,
                player_id,
                mode,
                record_id,
                record_id,
                filter_id,
                filter_id,
                player_id,
            )
            .fetch_one(&mut *conn)
            .await
            .map_err(SubmitRecordError::from)
            .map(|row| {
                let nub_rank = row.nub_rank.map(|rank| rank as u32);
                let nub_leaderboard_size = row.nub_leaderboard_size.map_or(0, |size| size as u32);
                let pro_rank = row.pro_rank.map(|rank| rank as u32);
                let pro_leaderboard_size = row.pro_leaderboard_size.map_or(0, |size| size as u32);
                let player_rating = match (row.nub_rating, row.pro_rating) {
                    (None, Some(_)) => unreachable!(),
                    (None, None) => 0.0,
                    (Some(nub_rating), None) => nub_rating,
                    // ?
                    (Some(nub_rating), Some(pro_rating)) => nub_rating + pro_rating,
                };

                SubmittedRecord {
                    record_id,
                    player_rating,
                    is_first_nub_record: old_nub.is_none(),
                    nub_rank,
                    nub_points: Option::zip(nub_rank, calc_nub_points)
                        .map(|(rank, calc_points)| calc_points((rank - 1) as usize)),
                    nub_leaderboard_size,
                    is_first_pro_record: old_pro.is_none(),
                    pro_rank,
                    pro_points: Option::zip(pro_rank, calc_pro_points)
                        .map(|(rank, calc_points)| calc_points((rank - 1) as usize)),
                    pro_leaderboard_size,
                }
            })
        })
        .await?;

    events::dispatch(Event::NewRecord {
        player_id,
        server_id,
        filter_id,
        styles,
        teleports,
        time,
        plugin_version_id,
        player_rating: record.player_rating,
        is_first_nub_record: record.is_first_nub_record,
        nub_rank: record.nub_rank,
        nub_points: record.nub_points,
        nub_leaderboard_size: record.nub_leaderboard_size,
        is_first_pro_record: record.is_first_pro_record,
        pro_rank: record.pro_rank,
        pro_points: record.pro_points,
        pro_leaderboard_size: record.pro_leaderboard_size,
    });

    Ok(record)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
    cx: &Context,
    GetRecordsParams {
        top,
        player_id,
        server_id,
        map_id,
        course_id,
        mode,
        has_teleports,
        limit,
        offset,
    }: GetRecordsParams,
) -> Result<Paginated<impl Stream<Item = Result<Record, GetRecordsError>>>, GetRecordsError> {
    macro_rules! count {
        ($table:literal $(, $extra:literal)? $(,)?) => {
            database::count!(
                cx.database().as_ref(),
                $table
                    + " JOIN CourseFilters AS cf ON cf.id = r.filter_id
                        JOIN Courses AS c ON c.id = cf.course_id
                        JOIN Maps AS m ON m.id = c.map_id
                        WHERE r.player_id = COALESCE(?, r.player_id)
                        AND r.server_id = COALESCE(?, r.server_id)
                        AND m.id = COALESCE(?, m.id)
                        AND c.id = COALESCE(?, c.id)
                        AND cf.mode = COALESCE(?, cf.mode)
                        AND r.teleports >= COALESCE(?, 0)
                        AND r.teleports <= COALESCE(?, (2 << 32) - 1)"
                    $( + $extra )?,
                player_id,
                server_id,
                map_id,
                course_id,
                mode,
                has_teleports.map(u8::from),
                has_teleports.map(|has_teleports| if has_teleports { u32::MAX } else { 0 }),
            )
        };
    }

    let total = if top {
        count!(
            "Records AS r
             LEFT JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
             LEFT JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id",
            "AND (NOT ((NubRecords.points IS NULL) AND (ProRecords.points IS NULL)))",
        )
        .await
    } else {
        count!("Records AS r").await
    }?;

    let records = self::macros::select!(
        "WHERE r.player_id = COALESCE(?, r.player_id)
         AND r.server_id = COALESCE(?, r.server_id)
         AND m.id = COALESCE(?, m.id)
         AND c.id = COALESCE(?, c.id)
         AND cf.mode = COALESCE(?, cf.mode)
         AND r.teleports >= COALESCE(?, 0)
         AND r.teleports <= COALESCE(?, (2 << 32) - 1)",
        player_id,
        server_id,
        map_id,
        course_id,
        mode,
        has_teleports.map(u8::from),
        has_teleports.map(|has_teleports| if has_teleports { u32::MAX } else { 0 });
        "WHERE (? OR (NubLeaderboard.rank >= 1 OR ProLeaderboard.rank >= 1))
         ORDER BY r.time ASC
         LIMIT ?
         OFFSET ?",
        !top,
        limit.value(),
        offset.value();
    )
    .fetch(cx.database().as_ref())
    .map_ok(move |row| {
        let mut record = self::macros::parse_row!(row);

        record.nub_points = record.nub_rank.map(|nub_rank| {
            points::complete(
                row.nub_tier,
                false,
                nub_rank as usize - 1,
                record.nub_points.unwrap_or_default(),
            )
        });

        record.pro_points = record.pro_rank.map(|pro_rank| {
            points::complete(
                row.pro_tier,
                true,
                pro_rank as usize - 1,
                record.pro_points.unwrap_or_default(),
            )
        });

        record
    })
    .map_err(GetRecordsError::from);

    Ok(Paginated::new(total, records))
}

#[tracing::instrument(skip(cx))]
pub fn get_player_records(
    cx: &Context,
    player_id: PlayerId,
    map_id: MapId,
) -> impl Stream<Item = Result<Record, GetRecordsError>> {
    self::macros::select!(
        "WHERE p.id = ? AND m.id = ?", player_id, map_id;
        "WHERE (NubLeaderboard.rank >= 1 OR ProLeaderboard.rank >= 1)";
    )
    .fetch(cx.database().as_ref())
    .map_ok(move |row| {
        let mut record = self::macros::parse_row!(row);

        record.nub_points = record.nub_rank.map(|nub_rank| {
            points::complete(
                row.nub_tier,
                false,
                nub_rank as usize - 1,
                record.nub_points.unwrap_or_default(),
            )
        });

        record.pro_points = record.pro_rank.map(|pro_rank| {
            points::complete(
                row.pro_tier,
                true,
                pro_rank as usize - 1,
                record.pro_points.unwrap_or_default(),
            )
        });

        record
    })
    .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx))]
pub fn get_leaderboard(
    cx: &Context,
    filter_id: CourseFilterId,
) -> impl Stream<Item = Result<LeaderboardEntry, GetRecordsError>> {
    sqlx::query_as!(
        LeaderboardEntry,
        "SELECT
           r.id AS `record_id: RecordId`,
           r.player_id AS `player_id: PlayerId`,
           r.teleports,
           r.time AS `time: Seconds`
         FROM Records AS r
         JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
         WHERE r.filter_id = ?
         ORDER BY r.time ASC, r.submitted_at ASC",
        filter_id,
    )
    .fetch(cx.database().as_ref())
    .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_by_id(
    cx: &Context,
    record_id: RecordId,
) -> Result<Option<Record>, GetRecordsError> {
    self::macros::select!("WHERE r.id = ?", record_id;)
        .fetch_optional(cx.database().as_ref())
        .await
        .map(|row| row.map(|row| self::macros::parse_row!(row)))
        .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_replay(
    cx: &Context,
    record_id: RecordId,
) -> Result<Option<Vec<u8>>, GetRecordsError> {
    sqlx::query_scalar!("SELECT data FROM RecordReplays WHERE record_id = ?", record_id)
        .fetch_optional(cx.database().as_ref())
        .await
        .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn count_by_filter(
    cx: &Context,
    filter_id: CourseFilterId,
) -> Result<u64, GetRecordsError> {
    database::count!(cx.database().as_ref(), "Records WHERE filter_id = ?", filter_id)
        .await
        .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx, records), err(level = "debug"))]
pub async fn update_best_records(
    cx: &Context,
    filter_id: CourseFilterId,
    records: impl IntoIterator<Item = BestRecord>,
) -> Result<(), GetRecordsError> {
    // This limit is fairly arbitrary and can be adjusted; we just don't want to exceed any query
    // length limits.
    const MAX_CHUNK_SIZE: usize = 1_000;

    let (nub, pro) = records
        .into_iter()
        .partition::<Vec<_>, _>(|record| record.nub_points != 0.0);

    let mut nub = nub.into_iter();
    let mut pro = pro.into_iter();

    cx.database_transaction(async move |conn| {
        let mut nub_query = QueryBuilder::new(
            "INSERT INTO BestNubRecords (
               filter_id,
               player_id,
               record_id,
               points
             )",
        );

        let mut pro_query = QueryBuilder::new(
            "INSERT INTO BestProRecords (
               filter_id,
               player_id,
               record_id,
               points,
               points_based_on_pro_leaderboard
             )",
        );

        while !(nub.as_slice().is_empty() && pro.as_slice().is_empty()) {
            if !nub.as_slice().is_empty() {
                nub_query.reset();
                nub_query.push_values(nub.by_ref().take(MAX_CHUNK_SIZE), |mut query, record| {
                    query.push_bind(filter_id);
                    query.push_bind(record.player_id);
                    query.push_bind(record.id);
                    query.push_bind(dbg!(record.nub_points));
                });

                nub_query.push(
                    "ON DUPLICATE KEY
                     UPDATE record_id = VALUES(record_id),
                            points = VALUES(points)",
                );

                nub_query
                    .build()
                    .persistent(false)
                    .execute(&mut *conn)
                    .await?;
            }

            if !pro.as_slice().is_empty() {
                pro_query.reset();
                pro_query.push_values(pro.by_ref().take(MAX_CHUNK_SIZE), |mut query, record| {
                    query.push_bind(filter_id);
                    query.push_bind(record.player_id);
                    query.push_bind(record.id);
                    query.push_bind(dbg!(record.pro_points.value));
                    query.push_bind(record.pro_points.based_on_pro_leaderboard);
                });

                pro_query.push(
                    "ON DUPLICATE KEY
                     UPDATE record_id = VALUES(record_id),
                            points = VALUES(points),
                            points_based_on_pro_leaderboard = VALUES(points_based_on_pro_leaderboard)",
                );

                pro_query
                    .build()
                    .persistent(false)
                    .execute(&mut *conn)
                    .await?;
            }
        }

        Ok(())
    })
    .await
}

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn delete(
    cx: &Context,
    filter_id: Option<CourseFilterId>,
    starting_at: Option<RecordId>,
    count: usize,
) -> database::Result<u64> {
    sqlx::query!(
        "DELETE FROM Records
         WHERE filter_id = COALESCE(?, filter_id)
         AND id >= COALESCE(?, 1)
         LIMIT ?",
        filter_id,
        starting_at,
        count as u64,
    )
    .execute(cx.database().as_ref())
    .await
    .map(|result| result.rows_affected())
    .map_err(database::Error::from)
}

mod macros {
    macro_rules! select {
        ( $inner:literal $(, $inner_args:expr)*; $( $outer:literal $(, $outer_args:expr)*; )?) => {
            sqlx::query!(
                "WITH NubLeaderboard AS (
                   SELECT
                     r.id AS record_id,
                     NubRecords.points,
                     RANK() OVER (
                       PARTITION BY r.filter_id
                       ORDER BY
                         r.time ASC,
                         r.submitted_at ASC
                     ) AS rank
                   FROM Records AS r
                   JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
                   JOIN Players AS p ON p.id = r.player_id
                   JOIN Servers AS s ON s.id = r.server_id
                   JOIN CourseFilters AS cf ON cf.id = r.filter_id
                   JOIN Courses AS c ON c.id = cf.course_id
                   JOIN Maps AS m ON m.id = c.map_id "
                + $inner
                + "),"
                + "ProLeaderboard AS (
                   SELECT
                     r.id AS record_id,
                     ProRecords.points,
                     RANK() OVER (
                       PARTITION BY r.filter_id
                       ORDER BY
                         r.time ASC,
                         r.submitted_at ASC
                     ) AS rank
                   FROM Records AS r
                   JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id
                   JOIN Players AS p ON p.id = r.player_id
                   JOIN Servers AS s ON s.id = r.server_id
                   JOIN CourseFilters AS cf ON cf.id = r.filter_id
                   JOIN Courses AS c ON c.id = cf.course_id
                   JOIN Maps AS m ON m.id = c.map_id "
                + $inner
                + ")
                   SELECT
                     r.id AS `id: RecordId`,
                     p.id AS `player_id: PlayerId`,
                     p.name AS player_name,
                     s.id AS `server_id: ServerId`,
                     s.name AS server_name,
                     m.id AS `map_id: MapId`,
                     m.name AS map_name,
                     c.id AS `course_id: CourseId`,
                     c.name AS course_name,
                     cf.mode AS `mode: Mode`,
                     cf.nub_tier AS `nub_tier: Tier`,
                     cf.pro_tier AS `pro_tier: Tier`,
                     r.styles AS `styles: Styles`,
                     r.teleports,
                     r.time AS `time: Seconds`,
                     NubLeaderboard.rank AS nub_rank,
                     NubLeaderboard.points AS nub_points,
                     ProLeaderboard.rank AS pro_rank,
                     ProLeaderboard.points AS pro_points,
                     r.submitted_at
                   FROM Records AS r
                   LEFT JOIN NubLeaderboard ON NubLeaderboard.record_id = r.id
                   LEFT JOIN ProLeaderboard ON ProLeaderboard.record_id = r.id
                   JOIN Players AS p ON p.id = r.player_id
                   JOIN Servers AS s ON s.id = r.server_id
                   JOIN CourseFilters AS cf ON cf.id = r.filter_id
                   JOIN Courses AS c ON c.id = cf.course_id
                   JOIN Maps AS m ON m.id = c.map_id "
                $( + $outer )?,
                $($inner_args,)*
                $($inner_args,)*
                $( $($outer_args,)* )?
            )
        };
    }

    macro_rules! parse_row {
        ($row:expr) => {
            Record {
                id: $row.id,
                player: PlayerInfo { id: $row.player_id, name: $row.player_name },
                server: ServerInfo { id: $row.server_id, name: $row.server_name },
                map: MapInfo { id: $row.map_id, name: $row.map_name },
                course: CourseInfo {
                    id: $row.course_id,
                    name: $row.course_name,
                    tier: if $row.teleports == 0 {
                        $row.pro_tier
                    } else {
                        $row.nub_tier
                    },
                },
                mode: $row.mode,
                styles: $row.styles,
                teleports: $row.teleports,
                time: $row.time,
                nub_rank: $row
                    .nub_rank
                    .map(|rank| rank.try_into().expect("rank should fit into u32")),
                nub_points: $row.nub_points,
                pro_rank: $row
                    .pro_rank
                    .map(|rank| rank.try_into().expect("rank should fit into u32")),
                pro_points: $row.pro_points,
                submitted_at: $row.submitted_at.into(),
            }
        };
    }

    pub(super) use {parse_row, select};
}
