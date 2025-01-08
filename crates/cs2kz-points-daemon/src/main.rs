#![feature(assert_matches)]
#![feature(thread_sleep_until)]

#[allow(unused_imports)]
#[macro_use(trace, debug, debug_span, info, info_span, warn, error, span)]
extern crate tracing;

use std::assert_matches::assert_matches;
use std::collections::hash_map::{self, HashMap};
use std::hash::{BuildHasher, Hash};
use std::sync::LazyLock;
use std::thread::{self, sleep_until};
use std::time::{Duration, Instant};
use std::{env, future};

use anyhow::Context as _;
use cs2kz::maps::CourseFilters;
use cs2kz::maps::courses::filters::{CourseFilterId, GetCourseFiltersParams};
use cs2kz::mode::Mode;
use cs2kz::points::{self, SMALL_LEADERBOARD_THRESHOLD};
use cs2kz::records::{BestRecord, ProPoints};
use futures_util::TryStreamExt;
use pyo3::Python;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::interval;
use tokio::{runtime, task};
use tracing::Instrument;

mod cli;

const THROTTLE: Duration = Duration::from_secs(5);

static TOKIO: LazyLock<runtime::Handle> = LazyLock::new(|| {
    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to construct tokio runtime");

    let handle = runtime.handle().clone();

    // make sure tasks continue to execute even if nobody else is blocking on `TOKIO.block_on`
    thread::spawn(move || {
        runtime.block_on(future::pending::<()>());
    });

    handle
});

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args = cli::args();
    let database_url = match args.database_url {
        Some(url) => url,
        None => env::var("DATABASE_URL")
            .context("missing `DATABASE_URL` environment variable or `--database-url` flag")?
            .parse()
            .context("failed to parse `DATABASE_URL`")?,
    };

    let config = cs2kz::Config {
        database: cs2kz::config::DatabaseConfig { url: database_url, ..Default::default() },
    };

    let cx = TOKIO
        .block_on(cs2kz::Context::new(config))
        .context("failed to initialize context")?;

    Python::with_gil(|py| run(py, cx))
}

fn run(py: Python<'_>, cx: cs2kz::Context) -> anyhow::Result<()> {
    let mut next_deadline = Instant::now() - THROTTLE;
    let mut filter_rx = fetch_filters(cx.clone())?;
    let mut filters = filter_rx
        .blocking_recv()
        .expect("task has not been cancelled")?;

    let mut record_counts_by_filter = HashMap::<CourseFilterId, u64>::with_capacity(filters.len());

    for filter_idx in 0_usize.. {
        next_deadline += THROTTLE;

        let filter_idx = filter_idx.checked_rem(filters.len());

        info!(
            sleeping_for =
                format_args!("~{:.2}s", (next_deadline - Instant::now()).as_secs_f64().round()),
            record_counts_by_filter.size = record_counts_by_filter.len(),
            ?filter_idx,
            "sleeping",
        );

        // throttle so we don't spin loop
        sleep_until(next_deadline);

        // check if we got new filters
        match filter_rx.try_recv() {
            Ok(new) => {
                filters = new?;
                info!(amount = filters.len(), "new filters");
            },
            Err(TryRecvError::Empty) => trace!("no new filters"),
            Err(TryRecvError::Disconnected) => unreachable!("task is not cancelled"),
        }

        let Some(filters) = filter_idx.map(|idx| &filters[idx]) else {
            debug!("no filters");
            continue;
        };

        for (mode, filter) in [
            (Mode::Vanilla, &filters.vanilla),
            (Mode::Classic, &filters.classic),
        ] {
            let span = info_span!("processing filter", %filter.id, ?mode);
            let _span_guard = span.enter();

            let record_count = TOKIO
                .block_on(cs2kz::records::count_by_filter(&cx, filter.id).in_current_span())
                .context("failed to get record count")?;

            debug!(record_count);

            // have new records been submitted?
            let count_changed = set_count(&mut record_counts_by_filter, filter.id, record_count);

            debug!(count_changed);

            if !count_changed {
                // nope! no need to recalc anything
                continue;
            }

            info!("getting leaderboard");

            let (nub_leaderboard, pro_leaderboard) = TOKIO.block_on(async {
                let mut nub_leaderboard = Vec::new();
                let mut pro_leaderboard = Vec::new();
                let mut records = cs2kz::records::get_leaderboard(&cx, filter.id);

                while let Some(record) = records
                    .try_next()
                    .await
                    .context("failed to get leaderboard")?
                {
                    nub_leaderboard.push(record);

                    if record.teleports == 0 {
                        pro_leaderboard.push(record);
                    }
                }

                anyhow::Ok((nub_leaderboard, pro_leaderboard))
            })?;

            info!(size = nub_leaderboard.len(), "calculating distribution for NUB leaderboard");

            let nub_dist = points::Distribution::new(py, &nub_leaderboard)
                .context("failed to calculate distribution parameters")?;

            info!(size = pro_leaderboard.len(), "calculating distribution for PRO leaderboard");

            let pro_dist = points::Distribution::new(py, &pro_leaderboard)
                .context("failed to calculate distribution parameters")?;

            info!("updating distribution data");

            TOKIO.block_on(async {
                cs2kz::points::update_distribution_data(&cx, filter.id, &nub_dist, &pro_dist)
                    .instrument(span.clone())
                    .await
                    .context("failed to save new distribution data")
            })?;

            let mut records = HashMap::new();

            let mut nub_dist_points_so_far = Vec::with_capacity(nub_leaderboard.len());
            let scaled_nub_times = nub_dist.scale(&nub_leaderboard).collect::<Vec<_>>();

            for (rank, entry) in nub_leaderboard.iter().enumerate() {
                let _guard = debug_span!(
                    "processing record",
                    leaderboard = "NUB",
                    distribution = "NUB",
                    %entry.record_id,
                    %entry.player_id,
                    %entry.teleports,
                    rank,
                )
                .entered();

                let points = if nub_leaderboard.len() <= SMALL_LEADERBOARD_THRESHOLD {
                    points::for_small_leaderboard(
                        filter.nub_tier,
                        nub_leaderboard[0].time.into(),
                        entry.time.into(),
                    )
                } else {
                    debug!(
                        leaderboard = "NUB",
                        distribution = "NUB",
                        "calculating points from distribution",
                    );

                    points::from_dist(
                        py,
                        &nub_dist,
                        &scaled_nub_times,
                        &nub_dist_points_so_far,
                        rank,
                    )
                    .inspect(|&points| nub_dist_points_so_far.push(points))
                    .map(|points| (points / nub_dist.top_scale).min(1.0))?
                };

                let old = records.insert(entry.record_id, BestRecord {
                    id: entry.record_id,
                    player_id: entry.player_id,
                    nub_points: points,
                    pro_points: ProPoints { value: 0.0, based_on_pro_leaderboard: false },
                });

                assert_matches!(old, None);
            }

            let mut pro_dist_points_so_far = Vec::with_capacity(pro_leaderboard.len());
            let scaled_pro_times = pro_dist.scale(&pro_leaderboard).collect::<Vec<_>>();

            for (rank, entry) in pro_leaderboard.iter().enumerate() {
                let _guard = debug_span!(
                    "processing record",
                    leaderboard = "NUB",
                    distribution = "NUB",
                    %entry.record_id,
                    %entry.player_id,
                    rank,
                )
                .entered();

                let pro_points = if pro_leaderboard.len() <= SMALL_LEADERBOARD_THRESHOLD {
                    Ok(points::for_small_leaderboard(
                        filter.pro_tier,
                        pro_leaderboard[0].time.into(),
                        entry.time.into(),
                    ))
                } else {
                    debug!(
                        leaderboard = "PRO",
                        distribution = "PRO",
                        "calculating points from distribution",
                    );

                    points::from_dist(
                        py,
                        &pro_dist,
                        &scaled_pro_times,
                        &pro_dist_points_so_far,
                        rank,
                    )
                    .inspect(|&points| pro_dist_points_so_far.push(points))
                    .map(|points| (points / pro_dist.top_scale).min(1.0))
                }?;

                // figure out where this record would be if placed in the nub leaderboard
                let (Ok(nub_rank) | Err(nub_rank)) =
                    nub_leaderboard.binary_search_by(|nub_record| nub_record.time.cmp(&entry.time));

                debug!(
                    leaderboard = "PRO",
                    distribution = "NUB",
                    "calculating points from distribution",
                );

                let nub_points = points::from_dist(
                    py,
                    &nub_dist,
                    &scaled_pro_times,
                    &nub_dist_points_so_far,
                    nub_rank,
                )?;

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
                    hash_map::Entry::Vacant(e) => {
                        e.insert(BestRecord {
                            id: entry.record_id,
                            player_id: entry.player_id,
                            nub_points: 0.0,
                            pro_points: points,
                        });
                    },
                    hash_map::Entry::Occupied(mut e) => {
                        e.get_mut().pro_points = points;
                    },
                }
            }

            info!("updating leaderboards");

            TOKIO.block_on(async {
                cs2kz::records::update_best_records(&cx, filter.id, records.into_values())
                    .instrument(span.clone())
                    .await
                    .context("failed to update leaderboards")?;

                anyhow::Ok(())
            })?;
        }
    }

    Ok(())
}

fn fetch_filters(
    cx: cs2kz::Context,
) -> anyhow::Result<mpsc::Receiver<anyhow::Result<Vec<CourseFilters>>>> {
    let (tx, rx) = mpsc::channel(16);
    let task = async move {
        let mut interval = interval(THROTTLE);

        loop {
            interval.tick().await;

            let filters = cs2kz::maps::courses::filters::get(&cx, GetCourseFiltersParams {
                approved_only: true,
                min_id: None,
            })
            .try_collect::<Vec<_>>()
            .await
            .context("failed to fetch filter data");

            info!(result = ?filters.as_ref().map(|filters| filters.len()), "fetched filters");

            if tx.send(filters).await.is_err() {
                warn!("receiver dropped; exiting");
                break;
            }
        }
    };

    task::Builder::new()
        .name("fetch-filters")
        .spawn_on(task.instrument(info_span!("fetch filters")), &TOKIO)
        .context("failed to spawn task")?;

    Ok(rx)
}

/// Equivalent to `counts[key] = count`, but returns whether the old value was equal to `count`.
fn set_count<K, S>(counts: &mut HashMap<K, u64, S>, key: K, count: u64) -> bool
where
    K: Eq + Hash,
    S: BuildHasher,
{
    use std::collections::hash_map::Entry;

    match counts.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(count);
            false
        },
        Entry::Occupied(mut entry) => {
            if *entry.get() == count {
                true
            } else {
                entry.insert(count);
                false
            }
        },
    }
}
