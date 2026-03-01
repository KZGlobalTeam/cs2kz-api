use std::num::NonZero;

use futures_util::{Stream, TryStreamExt};
use sqlx::{FromRow as _, Row as _};
use uuid::Uuid;

use crate::Context;
use crate::database::{self, QueryBuilder};
use crate::events::{self, Event};
use crate::maps::courses::filters::Tier;
use crate::maps::{CourseFilterId, CourseId, CourseInfo, MapId, MapInfo};
use crate::mode::Mode;
use crate::num::AsF64;
use crate::pagination::{Limit, Offset, Paginated};
use crate::players::{CalculateRatingError, PlayerId, PlayerInfo};
use crate::plugin::PluginVersionId;
use crate::points::calculator::{
    CalculatePointsError,
    Request as CalculatePointsRequest,
    Response as CalculatePointsResponse,
};
use crate::points::{self, DistributionParameters};
use crate::servers::{ServerId, ServerInfo};
use crate::styles::{ClientStyleInfo, Styles};
use crate::time::Seconds;

define_id_type! {
    /// A unique identifier for records.
    pub struct RecordId(Uuid);
}

impl RecordId {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

crate::database::impl_traits!(RecordId as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        let bytes = self.0.as_bytes();
        out = &bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        uuid::Bytes::try_from(bytes)
            .map(Uuid::from_bytes)
            .map(Self)
            .map_err(Into::into)
    }
});

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
    pub nub_max_rank: u32,
    pub nub_points: Option<f64>,
    pub pro_rank: Option<u32>,
    pub pro_max_rank: u32,
    pub pro_points: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct LeaderboardEntry {
    pub record_id: RecordId,
    pub player_id: PlayerId,
    pub teleports: u32,
    pub time: Seconds,
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
    pub styles: Vec<ClientStyleInfo>,
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

    /// The highest rank that any record should have.
    ///
    /// This can be used, for example, to query world records only (`max_rank=1`).
    pub max_rank: Option<NonZero<u32>>,

    /// Which value to sort the results by.
    pub sort_by: SortBy,

    /// Which direction to sort the results in.
    ///
    /// Defaults to 'descending' if `sort_by` is 'submission-date'.
    /// Defaults to 'ascending' if `sort_by` is 'time'.
    pub sort_order: Option<SortOrder>,

    pub limit: Limit<1000, 100>,
    pub offset: Offset,
}

#[derive(Debug, Default, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortBy {
    #[default]
    SubmissionDate,
    Time,
}

impl SortBy {
    fn sql(&self) -> &'static str {
        match self {
            Self::SubmissionDate => " r.id ",
            Self::Time => " r.time ",
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    fn from_sort_by(sort_by: SortBy) -> Self {
        match sort_by {
            SortBy::SubmissionDate => Self::Descending,
            SortBy::Time => Self::Ascending,
        }
    }

    fn sql(&self) -> &'static str {
        match self {
            Self::Ascending => " ASC ",
            Self::Descending => " DESC ",
        }
    }
}

#[derive(Debug)]
pub struct SubmittedRecord {
    pub record_id: RecordId,
    pub pb_data: Option<SubmittedPB>,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct SubmittedPB {
    pub nub_rank: Option<u32>,
    pub nub_points: Option<f64>,
    pub nub_leaderboard_size: u32,
    pub pro_rank: Option<u32>,
    pub pro_points: Option<f64>,
    pub pro_leaderboard_size: u32,
}

#[derive(Debug, Display, Error, From)]
pub enum SubmitRecordError {
    #[display("{_0}")]
    #[from]
    CalculatePoints(CalculatePointsError),

    #[display("{_0}")]
    #[from]
    CalculateRating(CalculateRatingError),

    #[display("{_0}")]
    #[from(database::Error, sqlx::Error)]
    Database(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get records: {_0}")]
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
        .database_transaction(async |conn| -> Result<_, SubmitRecordError> {
            let record_id = RecordId::new();

            sqlx::query!(
                "INSERT INTO Records (
                   id,
                   player_id,
                   server_id,
                   filter_id,
                   styles,
                   teleports,
                   time,
                   plugin_version_id
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                 record_id,
                player_id,
                server_id,
                filter_id,
                styles
                    .iter()
                    .map(|style_info| style_info.style)
                    .collect::<Styles>(),
                teleports,
                time,
                plugin_version_id,
            )
            .execute(&mut *conn)
            .await?;

            if !styles.is_empty() {
                return Ok(SubmittedRecord { record_id, pb_data: None });
            }

            let nub_pb_time = sqlx::query_scalar!(
                "SELECT
                  `time`
                 FROM BestNubRecords
                 WHERE filter_id = ?
                 AND player_id = ?",
                filter_id,
                player_id,
            )
            .fetch_optional(&mut *conn)
            .await?;

            let pro_pb_time = sqlx::query_scalar!(
                "SELECT
                   `time`
                 FROM BestProRecords
                 WHERE filter_id = ?
                 AND player_id = ?",
                filter_id,
                player_id,
            )
            .fetch_optional(&mut *conn)
            .await?;

            let nub_dist = sqlx::query_as!(
                DistributionParameters,
                "SELECT a, b, loc, scale, top_scale
                 FROM PointDistributionData
                 WHERE filter_id = ?
                 AND (NOT is_pro_leaderboard)",
                filter_id,
            )
            .fetch_optional(&mut *conn)
            .await?;

            let nub_tier = sqlx::query_scalar!(
                "SELECT nub_tier AS `tier: Tier`
                 FROM CourseFilters
                 WHERE id = ?",
                filter_id,
            )
            .fetch_one(&mut *conn)
            .await?;

            let (nub_leaderboard_size, nub_top_time) = sqlx::query!(
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
            .map(|row| row.map_or((0, None), |row| (row.size as u64, row.top_time)))?;

            let request = CalculatePointsRequest {
                time: time.0.as_secs_f64(),
                nub_data: points::calculator::LeaderboardData {
                    dist_params: nub_dist,
                    tier: nub_tier,
                    leaderboard_size: nub_leaderboard_size,
                    top_time: nub_top_time.unwrap_or(if let Some(nub_pb_time) = nub_pb_time {
                        time.as_f64().min(nub_pb_time)
                    } else {
                        time.as_f64()
                    }),
                },
                pro_data: {
                    if teleports == 0 {
                        let pro_dist = sqlx::query_as!(
                            DistributionParameters,
                            "SELECT a, b, loc, scale, top_scale
                             FROM PointDistributionData
                             WHERE filter_id = ?
                             AND (NOT is_pro_leaderboard)",
                            filter_id,
                        )
                        .fetch_optional(&mut *conn)
                        .await?;

                        let pro_tier = sqlx::query_scalar!(
                            "SELECT pro_tier AS `tier: Tier`
                             FROM CourseFilters
                             WHERE id = ?",
                            filter_id,
                        )
                        .fetch_one(&mut *conn)
                        .await?;

                        let (pro_leaderboard_size, pro_top_time) = sqlx::query!(
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
                        .map(|row| {
                            row.map_or((0, None), |row| (row.size as u64, row.top_time))
                        })?;

                        Some(points::calculator::LeaderboardData {
                            dist_params: pro_dist,
                            tier: pro_tier,
                            leaderboard_size: pro_leaderboard_size + u64::from(pro_pb_time.is_none()),
                            top_time: pro_top_time.unwrap_or(time.as_f64()),
                        })
                    } else {
                        None
                    }
                },
            };

            let points = if let Some(calc) = cx.points_calculator() {
                calc.calculate(request.clone()).await?
            } else {
                CalculatePointsResponse {
                    nub_fraction: points::for_small_leaderboard(
                        request.nub_data.tier,
                        request.nub_data.top_time,
                        request.time
                    ),
                    pro_fraction: request.pro_data.as_ref().map(|pro_data| {
                        points::for_small_leaderboard(
                            pro_data.tier,
                            pro_data.top_time,
                            request.time,
                        )
                    }),
                }
            };

            let is_nub_pb = nub_pb_time.is_none_or(|nub_pb_time| nub_pb_time > time.as_f64());

            if is_nub_pb {
                sqlx::query!(
                    "INSERT INTO BestNubRecords (
                       filter_id,
                       player_id,
                       record_id,
                       points,
                       time
                     )
                     VALUES (?, ?, ?, ?, ?)
                     ON DUPLICATE KEY
                     UPDATE record_id = VALUES(record_id),
                            points = VALUES(points),
                            time = VALUES(time)",
                    filter_id,
                    player_id,
                    record_id,
                    points.nub_fraction,
                    request.time,
                )
                .execute(&mut *conn)
                .await
                .inspect_err(|err| {
                    tracing::debug!(%filter_id, %player_id, dist_points = points.nub_fraction, %err);
                })?;
            }

            let is_pro_run = teleports == 0;
            let is_pro_pb = pro_pb_time.is_none_or(|pro_pb_time| pro_pb_time > time.as_f64());

            if is_pro_run && is_pro_pb {
                sqlx::query!(
                    "INSERT INTO BestProRecords (
                       filter_id,
                       player_id,
                       record_id,
                       points,
                       time
                     )
                     VALUES (?, ?, ?, ?, ?)
                     ON DUPLICATE KEY
                     UPDATE record_id = VALUES(record_id),
                            points = VALUES(points),
                            time = VALUES(time)",
                    filter_id,
                    player_id,
                    record_id,
                    points.pro_fraction.unwrap_or_else(|| {
                        tracing::warn!("no pro points for pro run?");
                        points.nub_fraction
                    }),
                    request.time,
                )
                .execute(&mut *conn)
                .await
                .inspect_err(|err| {
                    tracing::debug!(%filter_id, %player_id, dist_points = points.pro_fraction, %err);
                })?;
            }

            let ranks = self::macros::select_ranks_after_submit!(filter_id, player_id)
                .fetch_one(&mut *conn)
                .await?;

            let nub_rank = ranks.nub_rank.map(|rank| rank as u32);
            let nub_leaderboard_size = ranks.nub_leaderboard_size.map_or(0, |size| size as u32);
            let pro_rank = ranks.pro_rank.map(|rank| rank as u32);
            let pro_leaderboard_size = ranks.pro_leaderboard_size.map_or(0, |size| size as u32);

            Ok(SubmittedRecord {
                record_id,
                pb_data: Some(SubmittedPB {
                    nub_rank,
                    nub_points: nub_rank.map(|rank| {
                        points::complete(nub_tier, false, rank - 1, points.nub_fraction)
                    }),
                    nub_leaderboard_size,
                    pro_rank: pro_rank.filter(|_| teleports == 0),
                    pro_points: pro_rank
                        .zip(request.pro_data.as_ref())
                        .zip(points.pro_fraction)
                        .map(|((rank, data), dist_points)| {
                        points::complete(data.tier, true, rank - 1, dist_points)
                    }),
                    pro_leaderboard_size,
                }),
            })
        })
        .await?;

    cx.points_daemon().notify_record_submitted();

    events::dispatch(Event::NewRecord {
        player_id,
        server_id,
        filter_id,
        styles,
        teleports,
        time,
        plugin_version_id,
        pb_data: record.pb_data,
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
        max_rank,
        sort_by,
        sort_order,
        limit,
        offset,
    }: GetRecordsParams,
) -> Result<Paginated<Vec<Record>>, GetRecordsError> {
    fn base_filters(
        query: &mut QueryBuilder<'_>,
        map_id: Option<MapId>,
        course_id: Option<CourseId>,
        mode: Option<Mode>,
    ) {
        query.push(" WHERE m.id = COALESCE(");
        query.push_bind(map_id);
        query.push(", m.id) ");

        query.push(" AND c.id = COALESCE(");
        query.push_bind(course_id);
        query.push(", c.id) ");

        query.push(" AND cf.mode = COALESCE(");
        query.push_bind(mode);
        query.push(", cf.mode) ");
    }

    fn base_query(
        query: &mut QueryBuilder<'_>,
        map_id: Option<MapId>,
        course_id: Option<CourseId>,
        mode: Option<Mode>,
    ) {
        base_filters(query, map_id, course_id, mode);

        query.push("), ");
        query.push(
            "ProLeaderboard AS (
               SELECT
                 r.id AS record_id,
                 ProRecords.points,
                 RANK() OVER (
                   PARTITION BY r.filter_id
                   ORDER BY
                     r.time ASC,
                     r.id ASC
                 ) AS rank
               FROM Records AS r
               JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id
               JOIN Players AS p ON p.id = r.player_id
               JOIN Servers AS s ON s.id = r.server_id
               JOIN CourseFilters AS cf ON cf.id = r.filter_id
               JOIN Courses AS c ON c.id = cf.course_id
               JOIN Maps AS m ON m.id = c.map_id",
        );

        base_filters(query, map_id, course_id, mode);

        query.push(") ");
    }

    let total = {
        let mut query = QueryBuilder::new(
            "SELECT COUNT(r.id) AS total
             FROM Records AS r
             LEFT JOIN BestNubRecords ON BestNubRecords.record_id = r.id
             LEFT JOIN BestProRecords ON BestProRecords.record_id = r.id
             JOIN Servers AS s ON s.id = r.server_id
             JOIN CourseFilters AS cf ON cf.id = r.filter_id
             JOIN Courses AS c ON c.id = cf.course_id
             JOIN Maps AS m ON m.id = c.map_id",
        );

        base_filters(&mut query, map_id, course_id, mode);

        query.push(" AND r.player_id = COALESCE(");
        query.push_bind(player_id);
        query.push(", r.player_id) ");

        if top {
            match has_teleports {
                None | Some(true) => query.push(" AND (BestNubRecords.record_id IS NOT NULL) "),
                Some(false) => query.push(" AND (BestProRecords.record_id IS NOT NULL) "),
            };
        }

        query
            .build_query_scalar::<i64>()
            .fetch_one(cx.database().as_ref())
            .await?
            .try_into()
            .expect("`COUNT(…)` should not return a negative value")
    };

    let mut query = QueryBuilder::new(
        "WITH NubLeaderboard AS (
           SELECT
             r.id AS record_id,
             NubRecords.points,
             RANK() OVER (
               PARTITION BY r.filter_id
               ORDER BY
                 r.time ASC,
                 r.id ASC
             ) AS rank
           FROM Records AS r
           JOIN BestNubRecords AS NubRecords ON NubRecords.record_id = r.id
           JOIN Players AS p ON p.id = r.player_id
           JOIN Servers AS s ON s.id = r.server_id
           JOIN CourseFilters AS cf ON cf.id = r.filter_id
           JOIN Courses AS c ON c.id = cf.course_id
           JOIN Maps AS m ON m.id = c.map_id",
    );

    base_query(&mut query, map_id, course_id, mode);

    query.push(
        "SELECT
           r.id AS id,
           p.id AS player_id,
           p.name AS player_name,
           s.id AS server_id,
           s.name AS server_name,
           m.id AS map_id,
           m.name AS map_name,
           c.id AS course_id,
           c.name AS course_name,
           cf.mode AS mode,
           cf.nub_tier AS nub_tier,
           cf.pro_tier AS pro_tier,
           r.styles AS styles,
           r.teleports,
           r.time AS time,
           NubLeaderboard.rank AS nub_rank,
           COALESCE((SELECT COUNT(*) FROM NubLeaderboard), 0) AS nub_max_rank,
           NubLeaderboard.points AS nub_points,
           ProLeaderboard.rank AS pro_rank,
           COALESCE((SELECT COUNT(*) FROM ProLeaderboard), 0) AS pro_max_rank,
           ProLeaderboard.points AS pro_points
         FROM Records AS r
         LEFT JOIN NubLeaderboard ON NubLeaderboard.record_id = r.id
         LEFT JOIN ProLeaderboard ON ProLeaderboard.record_id = r.id
         JOIN Players AS p ON p.id = r.player_id
         JOIN Servers AS s ON s.id = r.server_id
         JOIN CourseFilters AS cf ON cf.id = r.filter_id
         JOIN Courses AS c ON c.id = cf.course_id
         JOIN Maps AS m ON m.id = c.map_id",
    );

    query.push(" AND p.id = COALESCE(");
    query.push_bind(player_id);
    query.push(", p.id) ");

    query.push(" AND s.id = COALESCE(");
    query.push_bind(server_id);
    query.push(", s.id) ");

    query.push(" AND m.id = COALESCE(");
    query.push_bind(map_id);
    query.push(", m.id) ");

    query.push(" AND c.id = COALESCE(");
    query.push_bind(course_id);
    query.push(", c.id) ");

    query.push(" AND cf.mode = COALESCE(");
    query.push_bind(mode);
    query.push(", cf.mode) ");

    if let Some(has_teleports) = has_teleports {
        query.push(" AND r.teleports ");
        query.push(if has_teleports { ">" } else { "=" });
        query.push(" 0");
    }

    if let Some(max_rank) = max_rank {
        query.push(" AND (NubLeaderboard.rank <= ");
        query.push_bind(max_rank.get());
        query.push(" OR ProLeaderboard.rank <= ");
        query.push_bind(max_rank.get());
        query.push(")");
    }

    if top {
        match has_teleports {
            None | Some(true) => query.push(" AND NubLeaderboard.rank >= 1 "),
            Some(false) => query.push(" AND ProLeaderboard.rank >= 1 "),
        };
    }

    query
        .push(" ORDER BY ")
        .push(sort_by.sql())
        .push(
            sort_order
                .unwrap_or_else(|| SortOrder::from_sort_by(sort_by))
                .sql(),
        )
        .push(", r.id ASC ")
        .push(" LIMIT ")
        .push_bind(limit.value())
        .push(" OFFSET ")
        .push_bind(offset.value());

    let records = query
        .build()
        .fetch(cx.database().as_ref())
        .map_err(GetRecordsError::from)
        .and_then(async move |row| {
            let mut record = Record::from_row(&row)?;
            let nub_tier = row.try_get::<Tier, _>("nub_tier")?;
            let pro_tier = row.try_get::<Tier, _>("pro_tier")?;

            record.nub_points = record.nub_rank.map(|nub_rank| {
                points::complete(
                    nub_tier,
                    false,
                    nub_rank - 1,
                    record.nub_points.unwrap_or_default(),
                )
            });

            record.pro_points = record.pro_rank.map(|pro_rank| {
                points::complete(
                    pro_tier,
                    true,
                    pro_rank - 1,
                    record.pro_points.unwrap_or_default(),
                )
            });

            Ok(record)
        })
        .try_collect()
        .await?;

    Ok(Paginated::new(total, records))
}

#[tracing::instrument(skip(cx))]
pub fn get_player_records(
    cx: &Context,
    player_id: PlayerId,
    map_id: MapId,
) -> impl Stream<Item = Result<Record, GetRecordsError>> {
    self::macros::select!(
        "WHERE m.id = ?", map_id;
        "WHERE p.id = ? AND (NubLeaderboard.rank >= 1 OR ProLeaderboard.rank >= 1)", player_id;
    )
    .fetch(cx.database().as_ref())
    .map_ok(move |row| {
        let mut record = self::macros::parse_row!(row);

        record.nub_points = record.nub_rank.map(|nub_rank| {
            points::complete(
                row.nub_tier,
                false,
                nub_rank - 1,
                record.nub_points.unwrap_or_default(),
            )
        });

        record.pro_points = record.pro_rank.map(|pro_rank| {
            points::complete(
                row.pro_tier,
                true,
                pro_rank - 1,
                record.pro_points.unwrap_or_default(),
            )
        });

        record
    })
    .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx))]
pub fn get_nub_leaderboard(
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
         ORDER BY r.time ASC, r.id ASC",
        filter_id,
    )
    .fetch(cx.database().as_ref())
    .map_err(GetRecordsError::from)
}

#[tracing::instrument(skip(cx))]
pub fn get_pro_leaderboard(
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
         JOIN BestProRecords AS ProRecords ON ProRecords.record_id = r.id
         WHERE r.filter_id = ?
         ORDER BY r.time ASC, r.id ASC",
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
    self::macros::select!("WHERE r.id = ?", record_id; "WHERE r.id = ?", record_id;)
        .fetch_optional(cx.database().as_ref())
        .await
        .map(|maybe_row| {
            maybe_row.map(|row| {
                let mut record = self::macros::parse_row!(row);

                record.nub_points = record.nub_rank.map(|nub_rank| {
                    points::complete(
                        row.nub_tier,
                        false,
                        nub_rank - 1,
                        record.nub_points.unwrap_or_default(),
                    )
                });

                record.pro_points = record.pro_rank.map(|pro_rank| {
                    points::complete(
                        row.pro_tier,
                        true,
                        pro_rank - 1,
                        record.pro_points.unwrap_or_default(),
                    )
                });

                record
            })
        })
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

impl<'r> sqlx::FromRow<'r, database::Row> for Record {
    fn from_row(row: &'r database::Row) -> sqlx::Result<Self> {
        let teleports = row.try_get("teleports")?;

        Ok(Self {
            id: row.try_get("id")?,
            player: PlayerInfo {
                id: row.try_get("player_id")?,
                name: row.try_get("player_name")?,
            },
            server: ServerInfo {
                id: row.try_get("server_id")?,
                name: row.try_get("server_name")?,
            },
            map: MapInfo {
                id: row.try_get("map_id")?,
                name: row.try_get("map_name")?,
            },
            course: CourseInfo {
                id: row.try_get("course_id")?,
                name: row.try_get("course_name")?,
                nub_tier: row.try_get("nub_tier")?,
                pro_tier: row.try_get("pro_tier")?,
            },
            mode: row.try_get("mode")?,
            styles: row.try_get("styles")?,
            teleports,
            time: row.try_get("time")?,
            nub_rank: match row.try_get::<Option<i64>, _>("nub_rank") {
                Ok(None) => None,
                Ok(Some(rank)) => {
                    Some(rank.try_into().map_err(|err| sqlx::Error::ColumnDecode {
                        index: String::from("nub_rank"),
                        source: Box::new(err),
                    })?)
                },
                Err(error) => return Err(error),
            },
            nub_max_rank: match row.try_get::<Option<i64>, _>("nub_max_rank") {
                Ok(None) => 0,
                Ok(Some(rank)) => rank.try_into().map_err(|err| sqlx::Error::ColumnDecode {
                    index: String::from("nub_max_rank"),
                    source: Box::new(err),
                })?,
                Err(error) => return Err(error),
            },
            nub_points: row.try_get("nub_points")?,
            pro_rank: match row.try_get::<Option<i64>, _>("pro_rank") {
                Ok(None) => None,
                Ok(Some(rank)) => {
                    Some(rank.try_into().map_err(|err| sqlx::Error::ColumnDecode {
                        index: String::from("pro_rank"),
                        source: Box::new(err),
                    })?)
                },
                Err(error) => return Err(error),
            },
            pro_max_rank: match row.try_get::<Option<i64>, _>("pro_max_rank") {
                Ok(None) => 0,
                Ok(Some(rank)) => rank.try_into().map_err(|err| sqlx::Error::ColumnDecode {
                    index: String::from("pro_max_rank"),
                    source: Box::new(err),
                })?,
                Err(error) => return Err(error),
            },
            pro_points: row.try_get("pro_points")?,
        })
    }
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
                         r.id ASC
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
                         r.id ASC
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
                     COALESCE((SELECT COUNT(*) FROM NubLeaderboard), 0) AS nub_max_rank,
                     NubLeaderboard.points AS nub_points,
                     ProLeaderboard.rank AS pro_rank,
                     COALESCE((SELECT COUNT(*) FROM ProLeaderboard), 0) AS pro_max_rank,
                     ProLeaderboard.points AS pro_points
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

    macro_rules! select_ranks_after_submit {
        ($filter_id:expr, $player_id:expr $(,)?) => {
            sqlx::query!(
                "WITH NubRecords AS (
                   SELECT
                     r.id AS record_id,
                     r.player_id,
                     cf.nub_tier AS tier,
                     BestNubRecords.points,
                     RANK() OVER (
                       PARTITION BY r.filter_id
                       ORDER BY
                         r.time ASC,
                         r.id ASC
                     ) AS rank
                   FROM Records AS r
                   JOIN BestNubRecords ON BestNubRecords.record_id = r.id
                   JOIN CourseFilters AS cf ON cf.id = r.filter_id
                   WHERE cf.id = ?
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
                         r.id ASC
                     ) AS rank
                   FROM Records AS r
                   JOIN BestProRecords ON BestProRecords.record_id = r.id
                   JOIN CourseFilters AS cf ON cf.id = r.filter_id
                   WHERE cf.id = ?
                 )
                 SELECT
                   (SELECT COUNT(*) FROM BestNubRecords WHERE filter_id = ?) AS nub_leaderboard_size,
                   (SELECT COUNT(*) FROM BestProRecords WHERE filter_id = ?) AS pro_leaderboard_size,
                   NubRecords.rank AS nub_rank,
                   ProRecords.rank AS pro_rank
                 FROM Players AS p
                 LEFT JOIN NubRecords ON NubRecords.player_id = p.id
                 LEFT JOIN ProRecords ON ProRecords.player_id = p.id
                 WHERE p.id = ?",
                $filter_id,
                $filter_id,
                $filter_id,
                $filter_id,
                $player_id,
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
                    nub_tier: $row.nub_tier,
                    pro_tier: $row.pro_tier,
                },
                mode: $row.mode,
                styles: $row.styles,
                teleports: $row.teleports,
                time: $row.time,
                nub_rank: $row
                    .nub_rank
                    .map(|rank| rank.try_into().expect("rank should fit into u32")),
                nub_max_rank: $row
                    .nub_max_rank
                    .map(|rank| rank.try_into().expect("rank should fit into u32"))
                    .unwrap_or_default(),
                nub_points: $row.nub_points,
                pro_rank: $row
                    .pro_rank
                    .map(|rank| rank.try_into().expect("rank should fit into u32")),
                pro_max_rank: $row
                    .pro_max_rank
                    .map(|rank| rank.try_into().expect("rank should fit into u32"))
                    .unwrap_or_default(),
                pro_points: $row.pro_points,
            }
        };
    }

    pub(super) use {parse_row, select, select_ranks_after_submit};
}
