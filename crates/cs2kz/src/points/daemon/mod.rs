use std::assert_matches::assert_matches;
use std::collections::hash_map::{self, HashMap};
use std::{future, iter};

use futures_util::{FutureExt, Stream, StreamExt, TryStreamExt};
use pyo3::PyErr;
use sqlx::QueryBuilder;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use self::record_counts::RecordCounts;
use crate::events::{self, Event};
use crate::maps::courses::filters::{
    self,
    CourseFilterId,
    GetCourseFiltersError,
    GetCourseFiltersParams,
};
use crate::points::{self, SMALL_LEADERBOARD_THRESHOLD, UpdateDistributionDataError};
use crate::records::{self, BestRecord, GetRecordsError, ProPoints};
use crate::{Context, database, maps, python};

mod record_counts;

#[derive(Debug, Display, Error, From)]
pub enum Error {
    GetCourseFilter(GetCourseFiltersError),
    GetRecords(GetRecordsError),
    #[error(ignore)]
    #[from(ignore)]
    Python(String),
    #[from(ignore)]
    GetCurrentRecordCounts(database::Error),
    #[from(ignore)]
    SaveFiltersToRecalculate(database::Error),
    UpdateDistributionData(UpdateDistributionDataError),
}

impl From<PyErr> for Error {
    fn from(err: PyErr) -> Self {
        // NOTE: we convert the error into a string eagerly because trying to do so without holding
        //       the GIL will cause a deadlock
        Self::Python(err.to_string())
    }
}

#[tracing::instrument(skip_all, err)]
pub async fn run(cx: Context, cancellation_token: CancellationToken) -> Result<(), Error> {
    let (filter_id_tx, mut filter_id_rx) = mpsc::channel(16);

    cx.spawn("record-count-tracker", |cancellation_token| {
        track_record_counts(cx.clone(), cancellation_token, filter_id_tx)
    });

    loop {
        select! {
            () = cancellation_token.cancelled() => {
                warn!("saving record counts");
                save_record_counts(&cx).await?;
                break;
            },

            result = filter_id_rx.recv() => match result {
                None => break,
                Some((filter_id, new_records)) => {
                    process_filter(&cx, filter_id, new_records).await?;
                },
            },
        }
    }

    Ok(())
}

#[tracing::instrument(
    skip(cx, filter_id),
    fields(id = %filter_id, mode = tracing::field::Empty),
    err,
)]
async fn process_filter(
    cx: &Context,
    filter_id: CourseFilterId,
    new_records: u64,
) -> Result<(), Error> {
    let span = tracing::Span::current();
    let (mode, filter) = filters::get_by_id(cx, filter_id)
        .await?
        .expect("there should be a valid filter with this ID");

    span.record("mode", tracing::field::debug(mode));
    info!("processing filter");

    let mut nub_leaderboard = Vec::new();
    let mut pro_leaderboard = Vec::new();
    let mut records = records::get_leaderboard(cx, filter.id);

    while let Some(record) = records.try_next().await? {
        nub_leaderboard.push(record);

        if record.teleports == 0 {
            pro_leaderboard.push(record);
        }
    }

    let (nub_dist, nub_leaderboard, pro_dist, pro_leaderboard) =
        python::execute(span.clone(), move |py| -> Result<_, Error> {
            info!(
                size = nub_leaderboard.len(),
                "calculating distribution parameters for the NUB leaderboard",
            );

            let nub_dist = points::Distribution::new(py, &nub_leaderboard)?;

            info!(
                size = pro_leaderboard.len(),
                "calculating distribution parameters for the PRO leaderboard",
            );

            let pro_dist = points::Distribution::new(py, &pro_leaderboard)?;

            info!("done calculating distribution parameters");

            Ok((nub_dist, nub_leaderboard, pro_dist, pro_leaderboard))
        })
        .await?;

    info!("updating distribution data");

    points::update_distribution_data(cx, filter.id, nub_dist.as_ref(), pro_dist.as_ref()).await?;

    info!("recalculating points");

    let records = python::execute(span.clone(), move |py| -> Result<_, Error> {
        let mut records = HashMap::new();
        let mut nub_dist_points_so_far = Vec::with_capacity(nub_leaderboard.len());
        let mut scaled_nub_times = Vec::with_capacity(nub_leaderboard.len());

        if let Some(ref nub_dist) = nub_dist {
            scaled_nub_times.extend(nub_dist.scale(&nub_leaderboard));

            for (rank, entry) in nub_leaderboard.iter().enumerate() {
                let _guard = debug_span!(
                    "processing record",
                    id = %entry.record_id,
                    player = %entry.player_id,
                    teleports = %entry.teleports,
                    rank,
                    leaderboard = "NUB",
                    distribution = "NUB",
                )
                .entered();

                let points = if nub_leaderboard.len() <= SMALL_LEADERBOARD_THRESHOLD {
                    points::for_small_leaderboard(
                        filter.nub_tier,
                        nub_leaderboard[0].time.into(),
                        entry.time.into(),
                    )
                } else {
                    debug!("calculating points from distribution");

                    points::from_dist(
                        py,
                        &nub_dist,
                        &scaled_nub_times,
                        &nub_dist_points_so_far,
                        rank,
                    )
                    .map(|points| {
                        nub_dist_points_so_far.push(points);
                        (points / nub_dist.top_scale).min(1.0)
                    })?
                };

                let slot = records.insert(entry.record_id, BestRecord {
                    id: entry.record_id,
                    player_id: entry.player_id,
                    nub_points: points,
                    pro_points: ProPoints::default(),
                });

                assert_matches!(slot, None);
            }
        }

        if let Some(ref pro_dist) = pro_dist {
            let nub_dist = nub_dist
                .as_ref()
                .expect("if there is a pro leaderboard, there mus also be a nub leaderboard");

            let mut pro_dist_points_so_far = Vec::with_capacity(pro_leaderboard.len());
            let scaled_pro_times = pro_dist.scale(&pro_leaderboard).collect::<Vec<_>>();

            for (rank, entry) in pro_leaderboard.iter().enumerate() {
                let span = debug_span!(
                    "processing record",
                    id = %entry.record_id,
                    player = %entry.player_id,
                    teleports = %entry.teleports,
                    rank,
                    leaderboard = "PRO",
                    distribution = "PRO",
                );
                let _guard = span.enter();

                let pro_points = if pro_leaderboard.len() <= SMALL_LEADERBOARD_THRESHOLD {
                    points::for_small_leaderboard(
                        filter.pro_tier,
                        pro_leaderboard[0].time.into(),
                        entry.time.into(),
                    )
                } else {
                    debug!("calculating points from distribution");

                    points::from_dist(
                        py,
                        &pro_dist,
                        &scaled_pro_times,
                        &pro_dist_points_so_far,
                        rank,
                    )
                    .map(|points| {
                        pro_dist_points_so_far.push(points);
                        (points / pro_dist.top_scale).min(1.0)
                    })?
                };

                let (Ok(rank_in_nub_leaderboard) | Err(rank_in_nub_leaderboard)) =
                    nub_leaderboard.binary_search_by(|nub_record| nub_record.time.cmp(&entry.time));

                span.record("distribution", "NUB");
                debug!("calculating points from distribution");

                let nub_points = if nub_leaderboard.len() <= SMALL_LEADERBOARD_THRESHOLD {
                    points::for_small_leaderboard(
                        filter.nub_tier,
                        nub_leaderboard[0].time.into(),
                        entry.time.into(),
                    )
                } else {
                    points::from_dist(
                        py,
                        nub_dist,
                        &scaled_nub_times,
                        &nub_dist_points_so_far,
                        rank_in_nub_leaderboard,
                    )
                    .map(|points| (points / nub_dist.top_scale).min(1.0))?
                };

                let points_based_on_pro_leaderboard = pro_points >= nub_points;
                let points = ProPoints {
                    value: if points_based_on_pro_leaderboard {
                        pro_points
                    } else {
                        nub_points
                    },
                    based_on_pro_leaderboard: points_based_on_pro_leaderboard,
                };

                match records.entry(entry.record_id) {
                    hash_map::Entry::Vacant(slot) => {
                        slot.insert(BestRecord {
                            id: entry.record_id,
                            player_id: entry.player_id,
                            nub_points: 0.0,
                            pro_points: points,
                        });
                    },
                    hash_map::Entry::Occupied(mut slot) => {
                        assert_eq!(slot.get().pro_points, ProPoints::default());
                        slot.get_mut().pro_points = points;
                    },
                }
            }
        }

        Ok(records)
    })
    .await?;

    info!("updating points");

    records::update_best_records(cx, filter.id, records.into_values()).await?;

    Ok(())
}

async fn track_record_counts(
    cx: Context,
    cancellation_token: CancellationToken,
    filter_id_tx: mpsc::Sender<(CourseFilterId, u64)>,
) -> Result<(), Error> {
    let mut permit = None;
    let mut record_counts = RecordCounts::new();

    let mut old_maps = events::subscribe();

    let filters_from_last_time = sqlx::query_scalar!(
        "SELECT filter_id AS `filter_id: CourseFilterId`
         FROM FiltersToRecalculate",
    )
    .fetch(cx.database().as_ref())
    .filter_map(|result| {
        future::ready(match result {
            Ok(filter_id) => Some(filter_id),
            Err(error) => {
                error!(%error, "failed to fetch filter id to recalculate from database");
                None
            },
        })
    });

    let filters_of_changed_record_counts = {
        let current_counts = sqlx::query!(
            "SELECT filter_id AS `filter_id!: CourseFilterId`, count FROM (
               SELECT
                 filter_id,
                 COUNT(*) OVER (PARTITION BY filter_id) AS count
               FROM Records
             ) AS _
             GROUP BY filter_id",
        )
        .fetch(cx.database().as_ref())
        .map_ok(|row| (row.filter_id, row.count as u64))
        .try_collect::<HashMap<_, _>>()
        .await
        .map_err(database::Error::from)
        .map_err(Error::GetCurrentRecordCounts)?;

        // TODO: technically, we should also include filters here that aren't in `RecordCounts` at
        //       all

        sqlx::query!(
            "SELECT
             filter_id AS `filter_id: CourseFilterId`,
             count
             FROM RecordCounts",
        )
        .fetch(cx.database().as_ref())
        .filter_map(move |row| {
            future::ready(match row {
                Ok(row) => current_counts
                    .get(&row.filter_id)
                    .is_none_or(|&current_count| current_count != row.count)
                    .then_some(row.filter_id),
                Err(error) => {
                    error!(%error, "failed to fetch record count from database");
                    None
                },
            })
        })
    };

    let new_filter_ids = events::subscribe().filter_map(|event| {
        future::ready(if let Event::NewRecord { filter_id, .. } = *event {
            Some(filter_id)
        } else {
            None
        })
    });

    let mut filter_ids = filters_from_last_time
        .chain(filters_of_changed_record_counts)
        .chain(new_filter_ids);

    loop {
        select! {
            () = cancellation_token.cancelled() => {
                warn!("saving outstanding filters to recalculate");
                save_filters_to_recalculate(&cx, filter_ids).await?;
                break Ok(());
            },

            Ok(new_permit) = filter_id_tx.reserve(), if permit.is_none() => {
                permit = Some(new_permit);
            },

            Some(filter_id) = filter_ids.next() => {
                record_counts.push(filter_id);

                if let Some(permit) = permit.take() {
                    permit.send(record_counts.pop().unwrap());
                }
            },

            Some(event) = old_maps.next() => {
                let Event::NewMap { ref name, .. } = *event else {
                    continue;
                };

                let mut maps = maps::get_by_name(&cx, name);

                while let Some(Ok(map)) = maps.next().await {
                    let mut filters = filters::get(&cx, GetCourseFiltersParams {
                        map_id: Some(map.id),
                        ..Default::default()
                    });

                    while let Some(Ok(filters)) = filters.next().await {
                        record_counts.remove(filters.vanilla.id);
                        record_counts.remove(filters.classic.id);
                    }
                }
            },
        }
    }
}

async fn save_record_counts(cx: &Context) -> Result<(), Error> {
    cx.database_transaction(async move |conn| {
        let counts = sqlx::query!(
            "SELECT * FROM (
               SELECT
                 filter_id AS id,
                 COUNT(*) OVER (PARTITION BY filter_id) AS count
               FROM Records
             ) AS _
             GROUP BY id",
        )
        .fetch_all(&mut *conn)
        .await?;

        if counts.is_empty() {
            return Ok(());
        }

        sqlx::query!("DELETE FROM RecordCounts")
            .execute(&mut *conn)
            .await?;

        let mut query = QueryBuilder::new("INSERT INTO RecordCounts (filter_id, count)");

        query.push_values(counts, |mut query, filter| {
            query.push_bind(filter.id);
            query.push_bind(filter.count);
        });

        query.push("ON DUPLICATE KEY UPDATE count = VALUES(count)");

        query.build().persistent(false).execute(&mut *conn).await?;

        Ok(())
    })
    .await
    .map_err(Error::SaveFiltersToRecalculate)
}

async fn save_filters_to_recalculate(
    cx: &Context,
    mut filters_to_recalculate: impl Stream<Item = CourseFilterId> + Unpin,
) -> Result<(), Error> {
    cx.database_transaction(async move |conn| {
        let mut query = QueryBuilder::new("INSERT IGNORE INTO FiltersToRecalculate ");
        let filter_ids = iter::from_fn(|| filters_to_recalculate.next().now_or_never());
        let mut empty = true;

        query.push_values(filter_ids, |mut query, filter_id| {
            empty = false;
            query.push_bind(filter_id);
        });

        if !empty {
            query.build().persistent(false).execute(&mut *conn).await?;
        }

        Ok(())
    })
    .await
    .map_err(Error::SaveFiltersToRecalculate)
}
