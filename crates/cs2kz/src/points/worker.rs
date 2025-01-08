use std::sync::{LazyLock, mpsc};
use std::thread;

use pyo3::{PyErr, Python};
use tokio::sync::oneshot;

use crate::maps::courses::filters::Tier;
use crate::points::{self, Distribution, SMALL_LEADERBOARD_THRESHOLD};

static WORKER: LazyLock<mpsc::Sender<Job>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        Python::with_gil(|py| {
            while let Ok(Job { response, params }) = rx.recv() {
                let _ = response.send(match (params.dist, params.leaderboard_size) {
                    (None, _) | (Some(_), ..=SMALL_LEADERBOARD_THRESHOLD) => {
                        Ok(points::for_small_leaderboard(params.tier, params.top_time, params.time))
                    },
                    (Some(dist), _) => try { dist.sf(py, params.time)? / dist.top_scale },
                });
            }
        })
    });

    tx
});

struct Job {
    response: oneshot::Sender<Result<f64, PyErr>>,
    params: CalculationParameters,
}

struct CalculationParameters {
    dist: Option<Distribution>,
    tier: Tier,
    leaderboard_size: usize,
    top_time: f64,
    time: f64,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to calculate points")]
pub struct CalculatePointsError(PyErr);

/// Calculates points for a new record with the given `time` at position `rank` in the
/// leaderboard.
///
/// # Panics
///
/// This function will panic if <code>tier > [Tier::Death]</code>.
pub async fn calculate(
    dist: Option<Distribution>,
    tier: Tier,
    leaderboard_size: usize,
    top_time: f64,
    time: f64,
) -> Result<f64, CalculatePointsError> {
    let (tx, rx) = oneshot::channel();
    let _ = WORKER.send(Job {
        response: tx,
        params: CalculationParameters { dist, tier, leaderboard_size, top_time, time },
    });

    rx.await
        .expect("worker died?")
        .map_err(CalculatePointsError)
}
