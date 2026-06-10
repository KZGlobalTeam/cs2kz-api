use crate::maps::courses::Tier;
use crate::records::RecordId;
use crate::nig;

pub mod daemon;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct NigData {
    pub a: f64,
    pub b: f64,
    pub loc: f64,
    pub scale: f64,
    pub top_scale: f64,
}

impl NigData {
    pub fn params(&self) -> nig::NigParams {
        nig::NigParams {
            a: self.a,
            b: self.b,
            loc: self.loc,
            scale: self.scale,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LeaderboardData {
    pub dist_params: Option<NigData>,
    #[serde(serialize_with = "Tier::serialize_as_integer")]
    pub tier: Tier,
    pub leaderboard_size: u64,
    #[serde(rename = "wr")]
    pub top_time: f64,
}

/// Input record data for batch recalculation.
#[derive(Debug, Clone)]
pub struct RecordTime {
    pub record_id: RecordId,
    pub time: f64,
}

/// Result of a leaderboard recalculation.
#[derive(Debug, Clone)]
pub struct RecalculatedLeaderboard {
    pub leaderboard: LeaderboardData,
    pub records: Vec<f64>,
}

/// The maximum points for any record.
pub const MAX: f64 = 10_000.0;

/// Threshold for what counts as a "small" leaderboard.
pub const SMALL_LEADERBOARD_THRESHOLD: u64 = 50;

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

pub fn calculate_fraction(time: f64, leaderboard: &LeaderboardData) -> f64 {
    if leaderboard.leaderboard_size < SMALL_LEADERBOARD_THRESHOLD {
        return for_small_leaderboard(leaderboard.tier, leaderboard.top_time, time);
    }

    let Some(dist) = leaderboard.dist_params else {
        return for_small_leaderboard(leaderboard.tier, leaderboard.top_time, time);
    };

    nig::sf(&dist.params(), time) / dist.top_scale
}

/// Recompute point fractions for a single leaderboard.
pub fn recalculate_leaderboard(
    records: &[RecordTime],
    tier: Tier,
    prev_params: Option<NigParams>,
) -> RecalculatedLeaderboard {
    let times: Vec<f64> = records.iter().map(|record| record.time).collect();
    let params = fit_distribution(&times, prev_params);

    let leaderboard = LeaderboardData {
        dist_params: params,
        tier,
        leaderboard_size: records.len() as u64,
        top_time: times.first().copied().unwrap_or(0.0),
    };

    let recalculated_records = records
        .iter()
        .map(|record| calculate_fraction(record.time, &leaderboard))
        .collect();

    RecalculatedLeaderboard { leaderboard, records: recalculated_records }
}

fn fit_distribution(times: &[f64], prev_params: Option<NigParams>) -> Option<NigData> {
    if times.len() < SMALL_LEADERBOARD_THRESHOLD as usize {
        return None;
    }

    let p = nig::fit(times, prev_params)?;
    let sf = nig::sf(&p, times[0]);
    let top_scale = if sf <= 0.0 { 1.0 } else { sf };

    Some(NigData {
        a: p.a,
        b: p.b,
        loc: p.loc,
        scale: p.scale,
        top_scale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_abs_close(actual: f64, expected: f64, tolerance: f64) {
        let abs_error = (actual - expected).abs();
        assert!(
            abs_error <= tolerance,
            "expected {expected:.15e}, got {actual:.15e}, abs error {abs_error:.2e}",
        );
    }

    #[test]
    fn calculate_fraction_matches_python_example() {
        let time = 8.609375;
        let nub_data = LeaderboardData {
            dist_params: Some(NigData {
                a: 33.53900289787477,
                b: 33.52140111667502,
                loc: 6.3663207368487065,
                scale: 0.4480388195262859,
                top_scale: 0.9979285278452101,
            }),
            tier: Tier::VeryEasy,
            leaderboard_size: 224,
            top_time: 7.6484375,
        };
        let pro_data = LeaderboardData {
            dist_params: Some(NigData {
                a: 2.6294814553333743,
                b: 2.511121972118702,
                loc: 8.713014153227697,
                scale: 2.2226724397990805,
                top_scale: 0.9952929135343108,
            }),
            tier: Tier::VeryEasy,
            leaderboard_size: 165,
            top_time: 7.6484375,
        };

        assert_abs_close(calculate_fraction(time, &nub_data), 0.9745534941686896, 1e-3);
        assert_abs_close(calculate_fraction(time, &pro_data), 0.9760910013054752, 1e-3);
    }

    #[test]
    fn calculate_fraction_falls_back_to_small_leaderboard_formula() {
        let leaderboard = LeaderboardData {
            dist_params: None,
            tier: Tier::VeryEasy,
            leaderboard_size: 30,
            top_time: 7.0,
        };

        assert_abs_close(
            calculate_fraction(8.0, &leaderboard),
            for_small_leaderboard(leaderboard.tier, leaderboard.top_time, 8.0),
            1e-12,
        );
    }
}
