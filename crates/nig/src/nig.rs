use serde::Serialize;

use crate::bessel::bessel_k1e;
use crate::differential_evo::differential_evolution;
use crate::nelder_mead::nelder_mead;
use crate::quad;

/// NIG distribution parameters (scipy loc-scale parameterization).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct NigParams {
    pub a: f64,
    pub b: f64,
    pub loc: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NigParamsReparametrized {
    pub log_a: f64,
    pub skew_raw: f64,
    pub loc: f64,
    pub log_scale: f64,
}

pub fn pdf(p: &NigParams, x: f64) -> f64 {
    if p.a <= 0.0 || p.scale <= 0.0 || p.b.abs() >= p.a {
        return 0.0;
    }

    let gamma = (p.a * p.a - p.b * p.b).sqrt();
    let z = (x - p.loc) / p.scale;
    let sqrt_z2p1 = (z * z + 1.0).sqrt();
    let y = p.a * sqrt_z2p1;
    let scaled_bessel = bessel_k1e(y);

    if scaled_bessel <= 0.0 {
        return 0.0;
    }

    let net_exp = gamma + p.b * z - y;
    let log_pdf = p.a.ln() - std::f64::consts::PI.ln() - p.scale.ln() - sqrt_z2p1.ln()
        + net_exp
        + scaled_bessel.ln();

    if log_pdf < -745.0 {
        // exp(-745) underflows to 0 in f64
        return 0.0;
    }

    log_pdf.exp()
}

pub fn cdf(p: &NigParams, x: f64) -> f64 {
    if p.a <= 0.0 || p.scale <= 0.0 || p.b.abs() >= p.a {
        return 0.0;
    }

    // The exp-sinh quadrature clusters its nodes near the finite endpoint, so
    // always integrate the side of the distribution whose mass lies closest to
    // `x`.
    // `loc + scale * b / a` is a cheap proxy for the mode: exact for b = 0 and
    // bounded by `loc ± scale`, unlike the mean which diverges as |b| -> a.
    let peak = p.loc + p.scale * p.b / p.a;

    if x <= peak {
        quad::quad(&mut |t| pdf(p, t), f64::NEG_INFINITY, x, 7, 1e-10, None).clamp(0.0, 1.0)
    } else {
        let tail = quad::quad(&mut |t| pdf(p, t), x, f64::INFINITY, 7, 1e-10, None);
        (1.0 - tail.clamp(0.0, 1.0)).clamp(0.0, 1.0)
    }
}

pub fn sf(p: &NigParams, x: f64) -> f64 {
    1.0 - cdf(p, x)
}

fn encode_nig_params(p: &NigParams) -> NigParamsReparametrized {
    let safe_a = p.a.max(1e-6);
    let safe_scale = p.scale.max(1e-6);
    let beta_ratio = (p.b / safe_a).clamp(-1.0 + 1e-12, 1.0 - 1e-12);
    NigParamsReparametrized {
        log_a: safe_a.ln(),
        skew_raw: beta_ratio.atanh(),
        loc: p.loc,
        log_scale: safe_scale.ln(),
    }
}

fn decode_nig_params(pr: &NigParamsReparametrized) -> NigParams {
    let a = pr.log_a.exp();
    let b = a * pr.skew_raw.tanh();
    let scale = pr.log_scale.exp();
    NigParams { a, b, loc: pr.loc, scale }
}

/// Moment-based initial parameter estimate for the NIG distribution.
fn estimate_nig_start(times: &[f64]) -> NigParams {
    let n = times.len() as f64;
    let mean = times.iter().sum::<f64>() / n;
    let m2 = times.iter().map(|&t| (t - mean).powi(2)).sum::<f64>() / n;

    if m2 < 1e-12 {
        return NigParams { a: 1.0, b: 0.0, loc: mean, scale: 1.0 };
    }

    let m3 = times.iter().map(|&t| (t - mean).powi(3)).sum::<f64>() / n;
    let m4 = times.iter().map(|&t| (t - mean).powi(4)).sum::<f64>() / n;
    let skewness = m3 / m2.powf(1.5);
    let excess_kurtosis = m4 / (m2 * m2) - 3.0;
    let denominator = excess_kurtosis - (4.0 * skewness * skewness) / 3.0;

    if denominator <= 1e-8 {
        let scale = m2.sqrt();
        return NigParams {
            a: (1.0 / scale).max(1e-3),
            b: 0.0,
            loc: mean,
            scale,
        };
    }

    let delta_gamma = 3.0 / denominator;
    let beta_ratio = (skewness * delta_gamma.sqrt() / 3.0).clamp(-1.0 + 1e-8, 1.0 - 1e-8);
    let cos_theta = (1.0 - beta_ratio * beta_ratio).max(1e-12).sqrt();
    let alpha = (delta_gamma / (m2 * cos_theta.powi(4))).max(1e-12).sqrt();
    let beta = alpha * beta_ratio;
    let scale = (delta_gamma / (alpha * cos_theta)).max(1e-12);
    let loc = mean - scale * beta_ratio / cos_theta;

    NigParams { a: alpha, b: beta, loc, scale }
}

fn neg_log_likelihood(times: &[f64], p: &NigParams) -> f64 {
    if p.a <= 0.0 || p.scale <= 0.0 || p.b.abs() >= p.a {
        return f64::INFINITY;
    }

    let gamma = (p.a * p.a - p.b * p.b).sqrt();
    let mut nll = 0.0;

    for &x in times {
        let z = (x - p.loc) / p.scale;
        let sqrt_z2p1 = (z * z + 1.0).sqrt();
        let y = p.a * sqrt_z2p1;
        let scaled_bessel = bessel_k1e(y);
        if scaled_bessel <= 0.0 {
            return f64::INFINITY;
        }
        let log_pdf =
            p.a.ln() - std::f64::consts::PI.ln() - p.scale.ln() - sqrt_z2p1.ln() + gamma + p.b * z
                - y
                + scaled_bessel.ln();
        nll -= log_pdf;
    }

    nll
}

fn de_bounds(times: &[f64]) -> [(f64, f64); 4] {
    let n = times.len() as f64;
    let mean = times.iter().sum::<f64>() / n;
    let variance = times.iter().map(|&t| (t - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt().max(1e-6);
    let data_range = (times.iter().fold(f64::INFINITY, |a, &b| a.min(b))
        - times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)))
    .abs()
    .max(1e-6);
    [
        (-2.0, 12.0),
        (-8.0, 8.0),
        (mean - 5.0 * std, mean + 5.0 * std),
        ((data_range / 100.0).ln(), (data_range * 100.0).ln()),
    ]
}

fn optimize(times: &[f64], inits: &[NigParams]) -> Result<(NigParams, usize), ()> {
    const MAX_ITER: usize = 1000;
    const TOL: f64 = 1e-4;
    const POLISH_MAX_ITER: usize = 2000;
    const POLISH_TOL: f64 = 1e-10;

    let bounds = de_bounds(times);

    let objective = |values: &[f64; 4]| -> f64 {
        let pr = NigParamsReparametrized {
            log_a: values[0],
            skew_raw: values[1],
            loc: values[2],
            log_scale: values[3],
        };
        neg_log_likelihood(times, &decode_nig_params(&pr))
    };

    let init_points: Vec<[f64; 4]> = inits
        .iter()
        .map(|p| {
            let pr = encode_nig_params(p);
            [pr.log_a, pr.skew_raw, pr.loc, pr.log_scale]
        })
        .collect();

    let (mut optimum, best_ll, mut nfev) = differential_evolution(
        objective,
        &bounds,
        TOL,
        MAX_ITER,
        15,
        (0.5, 1.0),
        0.7,
        0,
        &init_points,
    );

    if !best_ll.is_finite() {
        return Err(());
    }

    let (polished, polished_ll, polish_nfev) =
        nelder_mead(objective, &optimum, POLISH_MAX_ITER, POLISH_TOL);
    nfev += polish_nfev;

    if polished_ll.is_finite() && polished_ll < best_ll {
        optimum = polished;
    }

    let pr = NigParamsReparametrized {
        log_a: optimum[0],
        skew_raw: optimum[1],
        loc: optimum[2],
        log_scale: optimum[3],
    };

    Ok((decode_nig_params(&pr), nfev))
}

pub fn fit(times: &[f64], params: Option<NigParams>) -> NigParams {
    fit_with_stats(times, params).0
}

pub fn fit_with_stats(times: &[f64], params: Option<NigParams>) -> (NigParams, usize) {
    let moment_estimate = estimate_nig_start(times);

    let mut inits = vec![moment_estimate];
    if let Some(prev) = params
        && prev.a > 0.0
        && prev.scale > 0.0
        && prev.b.abs() < prev.a
    {
        inits.push(prev);
    }

    match optimize(times, &inits) {
        Ok((optimized, nfev)) => (optimized, nfev),
        Err(()) => {
            tracing::warn!(
                samples = times.len(),
                "NIG optimization failed; using initial estimates",
            );
            (moment_estimate, 0)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rel_close(actual: f64, expected: f64, tolerance: f64) {
        let rel_error = if expected == 0.0 {
            actual.abs()
        } else {
            (actual - expected).abs() / expected.abs()
        };
        assert!(
            rel_error <= tolerance,
            "expected {expected:.15e}, got {actual:.15e}, rel error {rel_error:.2e}",
        );
    }

    fn assert_abs_close(actual: f64, expected: f64, tolerance: f64) {
        let abs_error = (actual - expected).abs();
        assert!(
            abs_error <= tolerance,
            "expected {expected:.15e}, got {actual:.15e}, abs error {abs_error:.2e}",
        );
    }

    #[test]
    fn pdf_matches_reference_values() {
        let p = NigParams {
            a: 33.53900289787477,
            b: 33.52140111667502,
            loc: 6.3663207368487065,
            scale: 0.4480388195262859,
        };

        for (x, expected) in [
            (7.648, 9.314339782198335e-03),
            (8.0, 2.138356268395934e-02),
            (10.0, 7.240000069597700e-02),
            (20.0, 3.070336727949191e-02),
        ] {
            assert_rel_close(pdf(&p, x), expected, 1e-10);
        }
    }

    #[test]
    fn sf_matches_reference_values() {
        let p = NigParams {
            a: 33.53900289787477,
            b: 33.52140111667502,
            loc: 6.3663207368487065,
            scale: 0.4480388195262859,
        };

        for (x, expected) in [
            (7.0, 9.999892785756547e-01),
            (7.648, 9.979326056403205e-01),
            (10.0, 8.873317615160712e-01),
            (20.0, 3.429376452167427e-01),
        ] {
            assert_abs_close(sf(&p, x), expected, 1e-5);
        }
    }
}
