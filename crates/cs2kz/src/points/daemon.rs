use std::sync::Arc;
use std::time::Duration;
use std::{future, io};

use futures_util::TryFutureExt as _;
use tokio::sync::Notify;
use tokio::time::{interval, sleep};
use tokio_util::sync::CancellationToken;

use crate::maps::CourseFilterId;
use crate::maps::courses::filters::GetCourseFiltersError;
use crate::mode::Mode;
use crate::python::Python;
use crate::records::GetRecordsError;
use crate::{Context, database, players};

#[derive(Debug, Clone)]
pub struct PointsDaemonHandle {
    notifications: Arc<Notifications>,
}

impl PointsDaemonHandle {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Notifications { record_submitted: Notify::new() }),
        }
    }

    pub fn notify_record_submitted(&self) {
        self.notifications.record_submitted.notify_waiters();
    }
}

#[derive(Debug)]
struct Notifications {
    record_submitted: Notify,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    GetCourseFilter(GetCourseFiltersError),
    GetRecords(GetRecordsError),
    DetermineFilterToRecalculate(DetermineFilterToRecalculateError),
    Python(io::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to determine next filter to recalculate: {_0}")]
#[from(forward)]
pub struct DetermineFilterToRecalculateError(database::Error);

#[derive(Debug, serde::Serialize)]
struct PythonRequest {
    filter_id: CourseFilterId,
}

#[derive(Debug, serde::Deserialize)]
struct PythonResponse {
    #[expect(dead_code, reason = "included in tracing events")]
    filter_id: CourseFilterId,

    #[expect(dead_code, reason = "included in tracing events")]
    timings: PythonTimings,
}

#[derive(Debug, serde::Deserialize)]
#[expect(dead_code, reason = "included in tracing events")]
struct PythonTimings {
    #[serde(rename = "db_query_ms", deserialize_with = "deserialize_millis")]
    db_query: Duration,

    #[serde(rename = "nub_fit_ms", deserialize_with = "deserialize_millis")]
    nub_fit: Duration,

    #[serde(rename = "nub_compute_ms", deserialize_with = "deserialize_millis")]
    nub_compute: Duration,

    #[serde(rename = "pro_fit_ms", deserialize_with = "deserialize_millis")]
    pro_fit: Duration,

    #[serde(rename = "pro_compute_ms", deserialize_with = "deserialize_millis")]
    pro_compute: Duration,

    #[serde(rename = "db_write_ms", deserialize_with = "deserialize_millis")]
    db_write: Duration,
}

#[tracing::instrument(skip_all, err)]
pub async fn run(cx: Context, cancellation_token: CancellationToken) -> Result<(), Error> {
    let Some(script_path) = cx.config().points.calc_filter_path.as_deref() else {
        tracing::warn!("no `points.calc-filter-path` configured; points daemon will be disabled");
        return Ok(());
    };

    let mut python = Python::<PythonRequest, PythonResponse>::new(script_path.to_owned()).await?;
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
                process_filter(&mut python, &cancellation_token, filter_id).await?;
                update_filters_to_recalculate(&cx, filter_id, priority).await;
            },
        };
    }
}

#[tracing::instrument(skip(cx))]
async fn recalculate_ratings(cx: &Context) {
    use players::update_ratings;

    let (res1, res2) =
        future::join!(update_ratings(cx, Mode::Vanilla), update_ratings(cx, Mode::Classic)).await;

    if let Err(err) = res1 {
        tracing::error!(%err, mode = ?Mode::Vanilla, "failed to recalculate ratings");
    }

    if let Err(err) = res2 {
        tracing::error!(%err, mode = ?Mode::Classic, "failed to recalculate ratings");
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

        () = cx
            .points_daemon()
            .notifications
            .record_submitted
            .notified()
            .await;

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

#[tracing::instrument(skip(python))]
async fn process_filter(
    python: &mut Python<PythonRequest, PythonResponse>,
    cancellation_token: &CancellationToken,
    filter_id: CourseFilterId,
) -> Result<(), Error> {
    let request = PythonRequest { filter_id };

    loop {
        tracing::debug!(?request);
        match cancellation_token
            .run_until_cancelled(python.send_request(&request))
            .await
        {
            None => {
                tracing::debug!("cancelled");
                break Ok(());
            },
            Some(Ok(response)) => {
                tracing::debug!(?response);
                break Ok(());
            },
            Some(Err(err)) => {
                tracing::error!(%err, "failed to execute python request");
                python.reset().map_err(Error::Python).await?;
                sleep(Duration::from_secs(1)).await;
            },
        }
    }
}

fn deserialize_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <f64 as serde::Deserialize<'de>>::deserialize(deserializer)
        .map(|millis| millis / 1000.0)
        .map(Duration::from_secs_f64)
}
