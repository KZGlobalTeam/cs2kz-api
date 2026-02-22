use crate::maps::courses::Tier;

pub mod daemon;
pub mod calculator;

/// The maximum points for any record.
pub const MAX: f64 = 10_000.0;

/// Threshold for what counts as a "small" leaderboard.
pub const SMALL_LEADERBOARD_THRESHOLD: usize = 50;

/// [Normal-inverse Gaussian distribution][norminvgauss] parameters.
///
/// [norminvgauss]: https://en.wikipedia.org/wiki/Normal-inverse_Gaussian_distribution
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct DistributionParameters {
    pub a: f64,
    pub b: f64,
    pub loc: f64,
    pub scale: f64,
    pub top_scale: f64,
}

/// "Completes" pre-calculated distribution points cached in the database.
///
/// # Panics
///
/// This function will panic if <code>tier > [Tier::Death]</code>.
pub fn complete(tier: Tier, is_pro_leaderboard: bool, rank: u32, dist_points: f64) -> f64 {
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
pub fn for_rank(rank: u32) -> f64 {
    let mut points = 0.0;

    if rank < 100 {
        points += ((100 - rank) as f64) * 0.004;
    }

    if rank < 20 {
        points += ((20 - rank) as f64) * 0.02;
    }

    if let Some(&extra) = [0.2, 0.12, 0.09, 0.06, 0.02].get(rank as usize) {
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
