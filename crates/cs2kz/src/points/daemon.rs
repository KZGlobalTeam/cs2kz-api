use std::time::Duration;

use futures_util::TryFutureExt as _;
use nig::nig::NigParams;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::maps::CourseFilterId;
use crate::maps::courses::Tier;
use crate::mode::Mode;
use crate::players::PlayerId;
use crate::points::{self};
use crate::records::RecordId;
use crate::{Context, database, players};

const UPSERT_CHUNK_SIZE: usize = 5_000; // should prob put this somewhe

#[derive(Debug, Clone, Copy)]
struct BestRecordRow {
    filter_id: CourseFilterId,
    player_id: PlayerId,
    record_id: RecordId,
    time: f64,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    DetermineFilterToRecalculate(DetermineFilterToRecalculateError),
    ProcessFilter(database::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to determine next filter to recalculate: {_0}")]
#[from(forward)]
pub struct DetermineFilterToRecalculateError(database::Error);

#[tracing::instrument(skip_all, err)]
pub async fn run(cx: Context, cancellation_token: CancellationToken) -> Result<(), Error> {
    let mut recalc_ratings_interval = interval(Duration::from_secs(10));

    loop {
        select! {
            () = cancellation_token.cancelled() => {
                tracing::debug!("cancelled");
                break Ok(());
            },

            _ = recalc_ratings_interval.tick() => {
                tracing::debug!("recalculating ratings");
                recalculate_ratings(&cx).await;
            },

            res = determine_filter_to_recalculate(&cx) => {
                let (filter_id, priority) = res?;
                process_filter(&cx, filter_id).await?;
                update_filters_to_recalculate(&cx, filter_id, priority).await;
            },
        };
    }
}

#[tracing::instrument(skip(cx))]
async fn recalculate_ratings(cx: &Context) {
    use players::update_ratings;

    for mode in [Mode::Vanilla, Mode::Classic] {
        if let Err(err) = update_ratings(cx, mode).await {
            tracing::error!(%err, ?mode, "failed to recalculate ratings");
        }
    }
}

#[tracing::instrument(skip(cx))]
async fn determine_filter_to_recalculate(
    cx: &Context,
) -> Result<(CourseFilterId, u64), DetermineFilterToRecalculateError> {
    loop {
        if let Some(data) = sqlx::query!(
            "SELECT
               filter_id AS `filter_id: CourseFilterId`,
               priority
             FROM FiltersToRecalculate
             WHERE priority > 0
             ORDER BY priority DESC
             LIMIT 1",
        )
        .fetch_optional(cx.database().as_ref())
        .map_ok(|maybe_row| maybe_row.map(|row| (row.filter_id, row.priority)))
        .await?
        {
            break Ok(data);
        }

        cx.wait_for_points_recalculation().await;

        tracing::trace!("received notification about submitted record");
    }
}

#[tracing::instrument(skip(cx))]
async fn update_filters_to_recalculate(
    cx: &Context,
    filter_id: CourseFilterId,
    prev_priority: u64,
) {
    if let Err(err) = sqlx::query!(
        "UPDATE FiltersToRecalculate
         SET priority = (priority - ?)
         WHERE filter_id = ?",
        prev_priority,
        filter_id,
    )
    .execute(cx.database().as_ref())
    .await
    {
        tracing::warn!(%err, %filter_id, prev_priority, "failed to update FiltersToRecalculate");
    }
}

#[tracing::instrument(skip(cx))]
async fn process_filter(cx: &Context, filter_id: CourseFilterId) -> Result<(), database::Error> {
    tracing::debug!(%filter_id, "recalculating filter");

    let db = cx.database().as_ref();

    let nub_rows = sqlx::query_as!(
        BestRecordRow,
        "SELECT
           filter_id AS `filter_id: CourseFilterId`,
           player_id AS `player_id: PlayerId`,
           record_id AS `record_id: RecordId`,
           time
         FROM BestNubRecords
         WHERE filter_id = ?
         ORDER BY time ASC",
        filter_id,
    )
    .fetch_all(db)
    .await?;

    let nub_recs = nub_rows
        .iter()
        .map(|row| points::RecordTime { record_id: row.record_id, time: row.time })
        .collect::<Vec<_>>();

    // Pro records (sorted by time ASC)
    let pro_rows = sqlx::query_as!(
        BestRecordRow,
        "SELECT
           filter_id AS `filter_id: CourseFilterId`,
           player_id AS `player_id: PlayerId`,
           record_id AS `record_id: RecordId`,
           time
         FROM BestProRecords
         WHERE filter_id = ?
         ORDER BY time ASC",
        filter_id,
    )
    .fetch_all(db)
    .await?;

    let pro_recs = pro_rows
        .iter()
        .map(|row| points::RecordTime { record_id: row.record_id, time: row.time })
        .collect::<Vec<_>>();

    // Filter tiers
    let tiers_row = sqlx::query!(
        "SELECT
           nub_tier AS `nub_tier: Tier`,
           pro_tier AS `pro_tier: Tier`
         FROM CourseFilters
         WHERE id = ?",
        filter_id,
    )
    .fetch_optional(db)
    .await?;

    let Some(tiers_row) = tiers_row else {
        tracing::warn!(%filter_id, "filter not found in CourseFilters");
        return Ok(());
    };

    let nub_tier = tiers_row.nub_tier;
    let pro_tier = tiers_row.pro_tier;

    // Previous distribution parameters for warm start
    let prev_nub_params = sqlx::query_as!(
        NigParams,
        "SELECT a, b, loc, scale
         FROM PointDistributionData
         WHERE filter_id = ? AND (NOT is_pro_leaderboard)",
        filter_id,
    )
    .fetch_optional(db)
    .await?;

    let prev_pro_params = sqlx::query_as!(
        NigParams,
        "SELECT a, b, loc, scale
         FROM PointDistributionData
         WHERE filter_id = ? AND is_pro_leaderboard",
        filter_id,
    )
    .fetch_optional(db)
    .await?;

    let (nub_result, pro_result) = tokio::task::spawn_blocking(move || {
        let nub_result = points::recalculate_leaderboard(&nub_recs, nub_tier, prev_nub_params);

        let mut pro_result = points::recalculate_leaderboard(&pro_recs, pro_tier, prev_pro_params);

        for (record, recalculated_points) in pro_recs.iter().zip(pro_result.records.iter_mut()) {
            let nub_fraction = points::calculate_fraction(record.time, &nub_result.leaderboard);
            *recalculated_points = (*recalculated_points).max(nub_fraction);
        }

        (nub_result, pro_result)
    })
    .await
    .map_err(|_| {
        database::Error::decode(std::io::Error::other("points recalculation task panicked"))
    })?;

    tracing::debug!(
        %filter_id,
        nub_fitted = nub_result.leaderboard.dist_params.is_some(),
        pro_fitted = pro_result.leaderboard.dist_params.is_some(),
        "recalculation complete, writing to DB"
    );

    cx.database_transaction(async move |conn| -> Result<_, database::Error> {
        upsert_best_records(
            conn,
            "INSERT INTO BestNubRecords (filter_id, player_id, record_id, points, time)",
            &nub_rows,
            &nub_result.records,
        )
        .await?;

        upsert_best_records(
            conn,
            "INSERT INTO BestProRecords (filter_id, player_id, record_id, points, time)",
            &pro_rows,
            &pro_result.records,
        )
        .await?;

        if let Some(params) = nub_result.leaderboard.dist_params {
            sqlx::query!(
                "INSERT INTO PointDistributionData (
                    filter_id, is_pro_leaderboard, a, b, loc, scale, top_scale
                 )
                 VALUES (?, FALSE, ?, ?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE
                    a = VALUES(a),
                    b = VALUES(b),
                    loc = VALUES(loc),
                    scale = VALUES(scale),
                    top_scale = VALUES(top_scale)",
                filter_id,
                params.a,
                params.b,
                params.loc,
                params.scale,
                params.top_scale,
            )
            .execute(&mut *conn)
            .await?;
        }

        if let Some(params) = pro_result.leaderboard.dist_params {
            sqlx::query!(
                "INSERT INTO PointDistributionData (
                    filter_id, is_pro_leaderboard, a, b, loc, scale, top_scale
                 )
                 VALUES (?, TRUE, ?, ?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE
                    a = VALUES(a),
                    b = VALUES(b),
                    loc = VALUES(loc),
                    scale = VALUES(scale),
                    top_scale = VALUES(top_scale)",
                filter_id,
                params.a,
                params.b,
                params.loc,
                params.scale,
                params.top_scale,
            )
            .execute(&mut *conn)
            .await?;
        }

        Ok(())
    })
    .await?;

    Ok(())
}

async fn upsert_best_records(
    conn: &mut database::Connection,
    insert_prefix: &'static str,
    rows: &[BestRecordRow],
    recalculated_points: &[f64],
) -> Result<(), database::Error> {
    if rows.len() != recalculated_points.len() {
        return Err(database::Error::decode(std::io::Error::other(
            "recalculated record count does not match fetched best record rows",
        )));
    }

    for (row_chunk, points_chunk) in rows
        .chunks(UPSERT_CHUNK_SIZE)
        .zip(recalculated_points.chunks(UPSERT_CHUNK_SIZE))
    {
        let mut query = database::QueryBuilder::new(insert_prefix);

        query.push_values(row_chunk.iter().zip(points_chunk.iter()), |mut query, (row, points)| {
            query.push_bind(row.filter_id);
            query.push_bind(row.player_id);
            query.push_bind(row.record_id);
            query.push_bind(points);
            query.push_bind(row.time);
        });

        query.push(" ON DUPLICATE KEY UPDATE points = VALUES(points)");
        query.build().persistent(false).execute(&mut *conn).await?;
    }

    Ok(())
}
