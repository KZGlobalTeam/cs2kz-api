use std::iter;

use pyo3::PyErr;
use pyo3::types::{PyAnyMethods, PyTuple};

use crate::maps::courses::filters::{CourseFilterId, Tier};
use crate::python::PyCtx;
use crate::{Context, database, python};

mod distribution;
pub use distribution::Distribution;

pub mod daemon;

/// The maximum points for any record.
pub const MAX: f64 = 10_000.0;

/// Threshold for what counts as a "small" leaderboard.
pub const SMALL_LEADERBOARD_THRESHOLD: usize = 50;

#[derive(Debug, Display, Error)]
#[display("failed to calculate points: {_0}")]
#[error(ignore)]
pub struct CalculatePointsError(String);

impl From<PyErr> for CalculatePointsError {
    fn from(err: PyErr) -> Self {
        // NOTE: we convert the error into a string eagerly because trying to do so without holding
        //       the GIL will cause a deadlock
        Self(err.to_string())
    }
}

/// Calculates points for a new record with the given `time` at position `rank` in the
/// leaderboard.
///
/// # Panics
///
/// This function will panic if <code>tier > [Tier::Death]</code>.
pub fn calculate(
    dist: Option<Distribution>,
    tier: Tier,
    leaderboard_size: usize,
    top_time: f64,
    time: f64,
) -> impl Future<Output = Result<f64, CalculatePointsError>> {
    python::execute(tracing::Span::current(), move |py| match (dist, leaderboard_size) {
        (None, _) | (Some(_), ..=SMALL_LEADERBOARD_THRESHOLD) => {
            Ok(for_small_leaderboard(tier, top_time, time))
        },
        (Some(dist), _) => {
            let sf = dist.sf(py, time)?;

            if sf.is_nan() {
                warn!(?dist, leaderboard_size, top_time, time, "sf returned NaN");
                // return Ok(0.0);
            }

            Ok(sf / dist.top_scale)
        },
    })
}

/// "Completes" pre-calculated distribution points cached in the database.
///
/// # Panics
///
/// This function will panic if <code>tier > [Tier::Death]</code>.
pub fn complete(tier: Tier, is_pro_leaderboard: bool, rank: usize, dist_points: f64) -> f64 {
    let for_tier = for_tier(tier, is_pro_leaderboard);
    let remaining = MAX - for_tier;
    let for_rank = 0.125 * remaining * for_rank(rank);
    let from_dist = 0.875 * remaining * dist_points;

    for_tier + for_rank + from_dist
}

/// Calculates the amount of points to award for completing a difficult course.
///
/// # Panics
///
/// This function will panic if <code>tier > [Tier::Death]</code>.
pub const fn for_tier(tier: Tier, is_pro_leaderboard: bool) -> f64 {
    const POINTS_BY_TIER: [f64; 8] = [
        0.0, 500.0, 2_000.0, 3_500.0, 5_000.0, 6_500.0, 8_000.0, 9_500.0,
    ];

    const POINTS_BY_TIER_PRO: [f64; 8] = [
        1_000.0, 1_450.0, 2_800.0, 4_150.0, 5_500.0, 6_850.0, 8_200.0, 9_550.0,
    ];

    [POINTS_BY_TIER, POINTS_BY_TIER_PRO][is_pro_leaderboard as usize][(tier as usize) - 1]
}

/// Calculates the amount of points to award for achieving a high rank on the leaderboard.
pub fn for_rank(rank: usize) -> f64 {
    let mut points = 0.0;

    if rank < 100 {
        points += ((100 - rank) as f64) * 0.004;
    }

    if rank < 20 {
        points += ((20 - rank) as f64) * 0.02;
    }

    if let Some(&extra) = [0.2, 0.12, 0.09, 0.06, 0.02].get(rank) {
        points += extra;
    }

    points
}

/// Calculates the amount of points to award for completing a course only few others have also
/// completed.
///
/// The threshold for a "small" leaderboard is [`SMALL_LEADERBOARD_THRESHOLD`].
pub fn for_small_leaderboard(tier: Tier, top_time: f64, time: f64) -> f64 {
    // no idea what any of this means; consult zer0.k

    assert!(top_time <= time);

    let x = 2.1 - 0.25 * (tier as u8 as f64);
    let y = 1.0 + (x * -0.5).exp();
    let z = 1.0 + (x * (time / top_time - 1.5)).exp();

    y / z
}

/// Calculates the amount of points to award for perfoming relative to everyone else on the
/// leaderboard.
///
/// # Parameters
///
/// - `dist`: the distribution parameters calculated for the leaderboard
/// - `scaled_times`: leaderboard times scaled by [`Distribution::scale()`]
/// - `dist_points_so_far`: results returned by previous calls to this function
/// - `rank`: 0-indexed position of the record on the leaderboard
pub fn from_dist(
    cx: PyCtx<'_, '_>,
    dist: &Distribution,
    scaled_times: &[f64],
    dist_points_so_far: &[f64],
    rank: usize,
) -> Result<f64, PyErr> {
    // we already calculated this
    if rank == 0 {
        return Ok(dist.top_scale);
    }

    let curr_time = scaled_times[rank];
    let prev_time = scaled_times[rank - 1];

    // `rank` and `rank - 1` are tied, so just award the same points
    if curr_time == prev_time {
        return Ok(dist_points_so_far[rank - 1]);
    }

    let (diff, _) = cx
        .quad
        .call1((cx.pdf, prev_time, curr_time, (dist.a, dist.b)))?
        .downcast_into::<PyTuple>()?
        .extract::<(f64, f64)>()?;

    Ok(dist_points_so_far[rank - 1] - diff)
}

#[derive(Debug, Display, Error, From)]
#[display("failed to update point distribution data")]
#[from(forward)]
pub struct UpdateDistributionDataError(database::Error);

#[tracing::instrument(skip(cx), ret(level = "debug"), err(level = "debug"))]
pub async fn update_distribution_data(
    cx: &Context,
    filter_id: CourseFilterId,
    nub_dist: Option<&Distribution>,
    pro_dist: Option<&Distribution>,
) -> Result<(), UpdateDistributionDataError> {
    for (dist, is_pro_leaderboard) in
        iter::chain(nub_dist.map(|dist| (dist, false)), pro_dist.map(|dist| (dist, true)))
    {
        sqlx::query!(
            "INSERT INTO PointDistributionData (
               filter_id,
               is_pro_leaderboard,
               a,
               b,
               loc,
               scale,
               top_scale
             )
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY
             UPDATE a = VALUES(a),
                    b = VALUES(b),
                    loc = VALUES(loc),
                    scale = VALUES(loc),
                    top_scale = VALUES(top_scale)",
            filter_id,
            is_pro_leaderboard,
            dist.a,
            dist.b,
            dist.loc,
            dist.scale,
            dist.top_scale,
        )
        .execute(cx.database().as_ref())
        .await?;
    }

    Ok(())
}
