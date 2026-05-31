use crate::bessel::bessel_k1e;
use crate::params::NigParams;

pub(crate) fn nig_pdf(p: &NigParams, x: f64) -> f64 {
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

fn exp_sinh_g(
    m: i64,
    h_fine: f64,
    a: f64,
    half_pi: f64,
    cache: &mut std::collections::HashMap<i64, f64>,
    f: &mut impl FnMut(f64) -> f64,
) -> f64 {
    if let Some(&v) = cache.get(&m) {
        return v;
    }
    let t = (m as f64) * h_fine;
    let arg = half_pi * t.sinh();
    let v = if arg > 690.0 {
        0.0
    } else {
        let e = arg.exp(); // = x - a
        let x = a + e;
        let w = half_pi * t.cosh() * e; // Jacobian dx/dt
        let fv = f(x);
        if fv.is_finite() { w * fv } else { 0.0 }
    };
    cache.insert(m, v);
    v
}

// Integrate f over [a, +∞) using exp-sinh (double-exponential) quadrature
// with global step-halving (level doubling) and node reuse.
pub(crate) fn exp_sinh(
    f: &mut impl FnMut(f64) -> f64,
    a: f64,
    tol: f64,
    h0: f64,
    max_level: u32,
) -> f64 {
    let half_pi = std::f64::consts::PI / 2.0;
    let stride0 = 1i64 << max_level;
    let h_fine = h0 / (stride0 as f64);
    let cut = 1e-17_f64;
    let mut cache = std::collections::HashMap::<i64, f64>::new();

    let fmax0 = exp_sinh_g(0, h_fine, a, half_pi, &mut cache, f).abs();

    let ml = {
        let mut k = 0i64;
        loop {
            k -= 1;
            let m = k * stride0;
            let v = exp_sinh_g(m, h_fine, a, half_pi, &mut cache, f).abs();
            if k.abs() >= 2 && v <= cut * fmax0.max(v).max(1e-300) {
                break m;
            }
            if k.abs() > 64 {
                break m;
            }
        }
    };

    let mr = {
        let mut k = 0i64;
        loop {
            k += 1;
            let m = k * stride0;
            let v = exp_sinh_g(m, h_fine, a, half_pi, &mut cache, f).abs();
            if k.abs() >= 2 && v <= cut * fmax0.max(v).max(1e-300) {
                break m;
            }
            if k.abs() > 64 {
                break m;
            }
        }
    };

    let mut s = 0.0_f64;
    let mut m = ml;
    while m <= mr {
        s += exp_sinh_g(m, h_fine, a, half_pi, &mut cache, f);
        m += stride0;
    }
    let mut i_prev = h0 * s;
    let mut i = i_prev;

    for level in 1..=max_level {
        let stride = 1i64 << (max_level - level);
        let h = h0 / ((1u64 << level) as f64);
        let mut m = ml + stride;
        while m < mr {
            s += exp_sinh_g(m, h_fine, a, half_pi, &mut cache, f);
            m += 2 * stride;
        }
        i = h * s;
        let err = (i - i_prev).abs();
        if level >= 2 && err <= tol + tol * i.abs() {
            break;
        }
        i_prev = i;
    }

    i
}

pub fn nig_survival(p: &NigParams, x: f64) -> f64 {
    if p.a <= 0.0 || p.scale <= 0.0 || p.b.abs() >= p.a {
        return 0.0;
    }

    exp_sinh(&mut |t| nig_pdf(p, t), x, 1e-10, 0.5, 12).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::NigParams;

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
    fn nig_pdf_matches_reference_values() {
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
            assert_rel_close(nig_pdf(&p, x), expected, 1e-10);
        }
    }

    #[test]
    fn nig_survival_matches_reference_values() {
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
            assert_abs_close(nig_survival(&p, x), expected, 1e-5);
        }
    }
}
