use crate::bessel::bessel_k1e;
use crate::params::{NigParams, NigParamsReparametrized};

/// ln(f64::MAX): upper bound to avoid exp() overflow during optimization.
const MAX_LOG_VALUE: f64 = 709.782_712_893_384;

/// P1 Error: a combination of absolute and relative errors.
/// <https://arxiv.org/html/2403.07492v1>
fn p1_err(x: f64, y: f64) -> f64 {
    (x - y).abs() / (1.0 + y.abs())
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
        return NigParams { a: (1.0 / scale).max(1e-3), b: 0.0, loc: mean, scale };
    }

    let delta_gamma = 3.0 / denominator;
    let beta_ratio =
        (skewness * delta_gamma.sqrt() / 3.0).clamp(-1.0 + 1e-8, 1.0 - 1e-8);
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
        let log_pdf = p.a.ln() - std::f64::consts::PI.ln() - p.scale.ln()
            - sqrt_z2p1.ln()
            + gamma
            + p.b * z
            - y
            + scaled_bessel.ln();
        nll -= log_pdf;
    }

    nll
}

// Vector arithmetic helpers for [f64; 4] used in the Nelder-Mead simplex.
fn vsub(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}

fn vmadd(a: [f64; 4], s: f64, b: [f64; 4]) -> [f64; 4] {
    [a[0] + s * b[0], a[1] + s * b[1], a[2] + s * b[2], a[3] + s * b[3]]
}

/// Adaptive Nelder-Mead simplex optimizer.
///
/// Gao, F. and Han, L. (2012). Implementing the Nelder-Mead simplex algorithm
/// with adaptive parameters. *Computational Optimization and Applications*, 51:1, pp. 259–277.
fn nelder_mead(
    mut objective: impl FnMut(&[f64; 4]) -> f64, // hardcoded to 4 params
    initial: [f64; 4],
    tol: f64,
    max_iter: usize,
    bounds: &[(f64, f64); 4],
) -> ([f64; 4], f64, usize) {
    const N: usize = 4;
    let alpha = 1.0_f64;
    let beta = 1.0 + 2.0 / N as f64;
    let gamma = 0.75 - 1.0 / (2.0 * N as f64);
    let delta = 1.0 - 1.0 / N as f64;

    let clip = |x: [f64; N]| -> [f64; N] {
        let mut out = x;
        for (i, &(lo, hi)) in bounds.iter().enumerate() {
            out[i] = out[i].clamp(lo, hi);
        }
        out
    };

    let mut simplex = [[0.0_f64; N]; N + 1];
    let mut values = [0.0_f64; N + 1];
    let mut nfev: usize = 0;

    simplex[0] = initial;
    values[0] = { nfev += 1; objective(&simplex[0]) };

    for i in 0..N {
        let mut vertex = simplex[0];
        vertex[i] += if vertex[i] == 0.0 { 1.0 } else { vertex[i] * 0.05 };
        simplex[i + 1] = clip(vertex);
        values[i + 1] = { nfev += 1; objective(&simplex[i + 1]) };
    }

    for _ in 0..max_iter {
        // Sort simplex by ascending function value.
        let mut order = [0usize, 1, 2, 3, 4];
        order.sort_by(|&a, &b| {
            values[a]
                .partial_cmp(&values[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let sorted_s: [[f64; N]; N + 1] = std::array::from_fn(|i| simplex[order[i]]);
        let sorted_v: [f64; N + 1] = std::array::from_fn(|i| values[order[i]]);
        simplex = sorted_s;
        values = sorted_v;

        let best = simplex[0];
        let worst = simplex[N];

        // Centroid of all vertices except the worst.
        let mut centroid = [0.0_f64; N];
        for i in 0..N {
            for j in 0..N {
                centroid[j] += simplex[i][j];
            }
        }
        for j in 0..N {
            centroid[j] /= N as f64;
        }

        let reflected = clip(vmadd(centroid, alpha, vsub(centroid, worst)));
        let f_reflect = { nfev += 1; objective(&reflected) };

        if f_reflect < values[0] {
            let expanded = clip(vmadd(centroid, beta, vsub(centroid, worst)));
            let f_expand = { nfev += 1; objective(&expanded) };
            if f_expand < f_reflect {
                simplex[N] = expanded;
                values[N] = f_expand;
            } else {
                simplex[N] = reflected;
                values[N] = f_reflect;
            }
        } else if f_reflect < values[N - 1] {
            simplex[N] = reflected;
            values[N] = f_reflect;
        } else if f_reflect < values[N] {
            // Outside contraction.
            let xc = clip(vmadd(centroid, gamma, vsub(centroid, worst)));
            let f_contract = { nfev += 1; objective(&xc) };
            if f_contract <= f_reflect {
                simplex[N] = xc;
                values[N] = f_contract;
            } else {
                for i in 1..=N {
                    simplex[i] = clip(vmadd(best, delta, vsub(simplex[i], best)));
                    values[i] = { nfev += 1; objective(&simplex[i]) };
                }
            }
        } else {
            // Inside contraction.
            let xc = clip(vmadd(centroid, -gamma, vsub(centroid, worst)));
            let f_contract = { nfev += 1; objective(&xc) };
            if f_contract < values[N] {
                simplex[N] = xc;
                values[N] = f_contract;
            } else {
                for i in 1..=N {
                    simplex[i] = clip(vmadd(best, delta, vsub(simplex[i], best)));
                    values[i] = { nfev += 1; objective(&simplex[i]) };
                }
            }
        }

        // Convergence: max P1 error across all non-best vertices vs best.
        let max_simplex_err = (1..=N)
            .flat_map(|i| (0..N).map(move |j| p1_err(simplex[i][j], simplex[0][j])))
            .fold(f64::NEG_INFINITY, f64::max);
        let max_value_err = (1..=N)
            .map(|i| p1_err(values[i], values[0]))
            .fold(f64::NEG_INFINITY, f64::max);

        if max_simplex_err <= tol && max_value_err <= tol {
            break;
        }
    }

    let best_idx = values
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    (simplex[best_idx], values[best_idx], nfev)
}

fn optimize_nig(times: &[f64], p: &NigParams) -> Result<(NigParams, usize), ()> {
    const MAX_ITER: usize = 1000;
    const TOL: f64 = 1e-4;

    let n = times.len() as f64;
    let mean = times.iter().sum::<f64>() / n;
    let variance = times.iter().map(|&t| (t - mean).powi(2)).sum::<f64>() / n;
    let loc_step = variance.sqrt().max(1.0);

    let initial = encode_nig_params(p);
    let initial_arr = [initial.log_a, initial.skew_raw, initial.loc, initial.log_scale];

    let bounds = [
        (initial.log_a - 4.0, initial.log_a + 6.0),
        (initial.skew_raw - 4.0, initial.skew_raw + 4.0),
        (initial.loc - 10.0 * loc_step, initial.loc + 10.0 * loc_step),
        (initial.log_scale - 5.0, initial.log_scale + 5.0),
    ];

    let objective = |values: &[f64; 4]| -> f64 {
        if !values.iter().all(|v| v.is_finite()) {
            return f64::INFINITY;
        }
        if values[0] >= MAX_LOG_VALUE || values[3] >= MAX_LOG_VALUE {
            return f64::INFINITY;
        }
        let pr = NigParamsReparametrized {
            log_a: values[0],
            skew_raw: values[1],
            loc: values[2],
            log_scale: values[3],
        };
        neg_log_likelihood(times, &decode_nig_params(&pr))
    };

    let (optimum, best_ll, nfev) = nelder_mead(objective, initial_arr, TOL, MAX_ITER, &bounds);

    if !best_ll.is_finite() {
        return Err(());
    }

    let pr = NigParamsReparametrized {
        log_a: optimum[0],
        skew_raw: optimum[1],
        loc: optimum[2],
        log_scale: optimum[3],
    };

    Ok((decode_nig_params(&pr), nfev))
}

pub fn fit_nig(times: &[f64], params: Option<NigParams>) -> NigParams {
    let mut p = match params {
        Some(p) if p.a > 0.0 => p,
        _ => estimate_nig_start(times),
    };

    match optimize_nig(times, &p) {
        Ok((optimized, _nfev)) => p = optimized,
        Err(()) => {
            tracing::warn!(
                samples = times.len(),
                "NIG optimization failed; using initial estimates",
            );
        }
    }

    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{NigParams};

    fn assert_abs_close(actual: f64, expected: f64, tolerance: f64) {
        let abs_error = (actual - expected).abs();
        assert!(
            abs_error <= tolerance,
            "expected {expected:.15e}, got {actual:.15e}, abs error {abs_error:.2e}",
        );
    }

    /// Generates the 5 test datasets from the Python benchmark.
    fn test_datasets() -> Vec<Vec<f64>> {
        let linspace = |start: f64, end: f64, n: usize| -> Vec<f64> {
            (0..n).map(|i| start + (end - start) / (n - 1) as f64 * i as f64).collect()
        };
        vec![
            (0..200).map(|i: i32| 7.0 + (i as f64).powf(1.5) * 0.005).collect(),
            (0..200).map(|i: i32| 7.0 + (i as f64).powf(1.5) * 0.05).collect(),
            (0..200).map(|i: i32| 7.0 + (i as f64).powf(1.5) * 0.5).collect(),
            (0..50).map(|i: i32| 3.0 + (i as f64).powi(2) * 0.02).collect(),
            linspace(5.0, 35.0, 80),
        ]
    }

    #[test]
    fn neg_log_likelihood_returns_inf_for_invalid_params() {
        let times = [7.0, 8.0, 9.0, 10.0];
        assert!(neg_log_likelihood(
            &times,
            &NigParams { a: 0.0, b: 1.0, loc: 5.0, scale: 1.0 }
        )
        .is_infinite());
        assert!(neg_log_likelihood(
            &times,
            &NigParams { a: 2.0, b: 0.0, loc: 5.0, scale: 0.0 }
        )
        .is_infinite());
        assert!(neg_log_likelihood(
            &times,
            &NigParams { a: 1.0, b: 1.0, loc: 5.0, scale: 1.0 }
        )
        .is_infinite());
        assert!(neg_log_likelihood(
            &times,
            &NigParams { a: 1.0, b: -1.1, loc: 5.0, scale: 1.0 }
        )
        .is_infinite());
    }

    #[test]
    fn neg_log_likelihood_returns_finite_for_valid_params() {
        let times = [7.0, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0];
        let nll = neg_log_likelihood(
            &times,
            &NigParams { a: 5.0, b: 2.0, loc: 8.0, scale: 1.0 },
        );
        assert!(nll.is_finite());
        assert!(nll > 0.0);
    }

    #[test]
    fn neg_log_likelihood_matches_scipy_reference_value() {
        let times: Vec<f64> =
            (0..200).map(|i: i32| 7.0 + (i as f64).powf(1.5) * 0.005).collect();
        let nll = neg_log_likelihood(
            &times,
            &NigParams {
                a: 86.72396846356486,
                b: 86.68319372270773,
                loc: 4.487105095426718,
                scale: 0.24914444085073106,
            },
        );
        assert_abs_close(nll, 559.6276051205211, 1e-6);
    }

    #[test]
    fn estimate_nig_start_produces_valid_params() {
        let times: Vec<f64> =
            (0..200).map(|i: i32| 7.0 + (i as f64).powf(1.5) * 0.005).collect();
        let p = estimate_nig_start(&times);
        assert!(p.a > 0.0, "a must be positive");
        assert!(p.scale > 0.0, "scale must be positive");
        assert!(p.b.abs() < p.a, "|b| must be < a");
    }

    #[test]
    fn encode_decode_roundtrips() {
        let p = NigParams { a: 5.0, b: 2.0, loc: 8.0, scale: 1.5 };
        let decoded = decode_nig_params(&encode_nig_params(&p));
        assert_abs_close(decoded.a, p.a, 1e-12);
        assert_abs_close(decoded.b, p.b, 1e-12);
        assert_abs_close(decoded.loc, p.loc, 1e-12);
        assert_abs_close(decoded.scale, p.scale, 1e-12);
    }

    #[test]
    fn fit_nig_matches_python_reference_all_datasets() {
        struct Ref {
            a: f64,
            b: f64,
            loc: f64,
            scale: f64,
            nll: f64,
            nfev: usize,
        }
        let refs = [
            Ref { a: 72.467, b: 72.418, loc: 4.491, scale: 0.298, nll: 559.637, nfev: 354 },
            Ref { a: 9.536, b: 9.276, loc: -11.183, scale: 17.732, nll: 1021.770, nfev: 685 },
            Ref { a: 0.954, b: 0.650, loc: 232.337, scale: 361.840, nll: 1502.561, nfev: 488 },
            Ref { a: 21.769, b: 21.754, loc: 0.365, scale: 0.687, nll: 195.001, nfev: 468 },
            Ref { a: 46.005, b: -0.001, loc: 20.001, scale: 59.861, nll: 287.470, nfev: 831 },
        ];

        for (idx, (times, r)) in test_datasets().iter().zip(&refs).enumerate() {
            let ds = idx + 1;
            let initial = estimate_nig_start(times);
            let (p, nfev) = optimize_nig(times, &initial)
                .unwrap_or_else(|_| panic!("ds{ds}: optimization failed"));
            let nll = neg_log_likelihood(times, &p);
            let tol = 1e-3;
            assert!((p.a - r.a).abs() <= tol, "ds{ds} a: got {:.3}, expected {:.3}", p.a, r.a);
            assert!((p.b - r.b).abs() <= tol, "ds{ds} b: got {:.3}, expected {:.3}", p.b, r.b);
            assert!(
                (p.loc - r.loc).abs() <= tol,
                "ds{ds} loc: got {:.3}, expected {:.3}",
                p.loc, r.loc,
            );
            assert!(
                (p.scale - r.scale).abs() <= tol,
                "ds{ds} scale: got {:.3}, expected {:.3}",
                p.scale, r.scale,
            );
            assert!(
                (nll - r.nll).abs() <= tol,
                "ds{ds} nll: got {:.3}, expected {:.3}",
                nll, r.nll,
            );
            assert_eq!(nfev, r.nfev, "ds{ds}: nfev");
        }
    }

    #[test]
    fn fit_nig_warm_start_from_previous_result() {
        let times: Vec<f64> =
            (0..200).map(|i: i32| 7.0 + (i as f64).powf(1.5) * 0.005).collect();
        let cold = fit_nig(&times, None);
        let warm = fit_nig(&times, Some(cold));
        let cold_nll = neg_log_likelihood(&times, &cold);
        let warm_nll = neg_log_likelihood(&times, &warm);
        // Warm start should reach same or better NLL.
        assert!(
            warm_nll <= cold_nll + 1e-4,
            "warm_nll={warm_nll:.6}, cold_nll={cold_nll:.6}",
        );
    }
}
