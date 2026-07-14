// BSD 3-Clause License

// Copyright (c) 2022, Robert A. van Engelen
// All rights reserved.

// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:

// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.

// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.

// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

// article: https://www.genivia.com/qthsh.html
// original code: https://github.com/Robert-van-Engelen/Tanh-Sinh

const FUDGE1: f64 = 10.0;
const FUDGE2: f64 = 1.0;

fn exp_sinh_opt_d(f: &mut impl FnMut(f64) -> f64, a: f64, eps: f64, mut d: f64) -> f64 {
    let mut _ev = 2;
    // const base = 2; // 2 or 3 or exp(1) for example
    let h2 = f(a + d / 2.0) - f(a + d * 2.0) * 4.0;
    let mut i = 1;
    let mut j = 32; // j=32 is optimal to search for r

    if h2.is_finite() && h2.abs() > 1e-5 {
        // if |h2| > 2^-16
        let mut fl: f64;
        let mut fr: f64;
        let mut h: f64;
        let mut s = 0.0;
        let mut lfl: f64;
        let mut lfr: f64;
        let mut lr = 2.0;

        // find max j such that fl and fr are finite
        loop {
            j /= 2;
            let r = (1 << (i + j)) as f64;
            fl = f(a + d / r);
            fr = f(a + d * r) * r * r;
            _ev += 2;
            h = fl - fr;
            if j <= 1 || h.is_finite() {
                break;
            }
        }

        if j > 1 && h.is_finite() && h.signum() != h2.signum() {
            lfl = fl; // last fl=f(a+d/r)
            lfr = fr; // last fr=f(a+d*r)*r*r

            // bisect in 4 iterations
            loop {
                j /= 2;
                let r = (1 << (i + j)) as f64;
                fl = f(a + d / r);
                fr = f(a + d * r) * r * r;
                _ev += 2;
                h = fl - fr;
                if h.is_finite() {
                    s += h.abs(); // sum |h| to remove noisy cases
                    if h.signum() == h2.signum() {
                        i += j; // search right half
                    } else {
                        // search left half
                        lfl = fl; // record last fl=f(a+d/r)
                        lfr = fr; // record last fr=f(a+d*r)*r*r
                        lr = r; // record last r
                    }
                }
                if j <= 1 {
                    break;
                }
            }

            if s > eps {
                // if sum of |h| > eps
                h = lfl - lfr; // use last fl and fr before the sign change
                let mut r = lr; // use last r before the sign change
                if h != 0.0 {
                    // if last difference was nonzero, back up r by one step
                    r /= 2.0;
                }
                if lfl.abs() < lfr.abs() {
                    d /= r; // move d closer to the finite endpoint
                } else {
                    d *= r; // move d closer to the infinite endpoint
                }
            }
        }
    }

    d
}

/// Integrate function `f` over the range `a..b`.
///
/// `n` is the max number of levels (2 to 7, 6 is recommended).
/// `eps` is the relative error tolerance.
/// If `err` is `Some`, the estimated relative error is written to it.
pub fn quad(f: &mut impl FnMut(f64) -> f64, a: f64, b: f64, n: i32, eps: f64, err: Option<&mut f64>) -> f64 {
    let tol = FUDGE1 * eps;
    let mut c = 0.0;
    let mut d = 1.0;
    let mut sign = 1.0;
    let mut h: f64 = 2.0;
    let mut k = 0;
    let mut mode = 0; // Tanh-Sinh = 0, Exp-Sinh = 1, Sinh-Sinh = 2

    let (mut a, mut b) = (a, b);
    if b < a {
        // swap bounds
        let v = b;
        b = a;
        a = v;
        sign = -1.0;
    }

    let mut v: f64;
    if a.is_finite() && b.is_finite() {
        c = (a + b) / 2.0;
        d = (b - a) / 2.0;
        v = c;
    } else if a.is_finite() {
        mode = 1; // Exp-Sinh
        d = exp_sinh_opt_d(f, a, eps, d);
        c = a;
        v = a + d;
    } else if b.is_finite() {
        mode = 1; // Exp-Sinh
        d = exp_sinh_opt_d(f, b, eps, -d);
        sign = -sign;
        c = b;
        v = b + d;
    } else {
        mode = 2; // Sinh-Sinh
        v = 0.0;
    }

    let mut s = f(v);

    loop {
        let mut p = 0.0;
        let mut q: f64;
        let mut fp = 0.0;
        let mut fm = 0.0;
        let mut t: f64;
        let mut eh: f64;

        h /= 2.0;
        t = h.exp();
        eh = t;
        if k > 0 {
            eh *= eh;
        }

        if mode == 0 {
            // Tanh-Sinh
            loop {
                let u = (1.0 / t - t).exp(); // = exp(-2*sinh(j*h)) = 1/exp(sinh(j*h))^2
                let r = 2.0 * u / (1.0 + u); // = 1 - tanh(sinh(j*h))
                let w = (t + 1.0 / t) * r / (1.0 + u); // = cosh(j*h)/cosh(sinh(j*h))^2
                let x = d * r;

                if a + x > a {
                    // if too close to a then reuse previous fp
                    let y = f(a + x);
                    if y.is_finite() {
                        fp = y; // if f(x) is finite, add to local sum
                    }
                }
                if b - x < b {
                    // if too close to b then reuse previous fm
                    let y = f(b - x);
                    if y.is_finite() {
                        fm = y; // if f(x) is finite, add to local sum
                    }
                }

                q = w * (fp + fm);
                p += q;
                t *= eh;
                if q.abs() <= eps * p.abs() {
                    break;
                }
            }
        } else {
            t /= 2.0;
            loop {
                let mut r = (t - 0.25 / t).exp(); // = exp(sinh(j*h))
                let mut x: f64;
                let mut y: f64;
                let mut w = r;
                q = 0.0;

                if mode == 1 {
                    // Exp-Sinh
                    x = c + d / r;
                    if x == c {
                        // if x hit the finite endpoint then break
                        break;
                    }
                    y = f(x);
                    if y.is_finite() {
                        // if f(x) is finite, add to local sum
                        q += y / w;
                    }
                } else {
                    // Sinh-Sinh
                    r = (r - 1.0 / r) / 2.0; // = sinh(sinh(j*h))
                    w = (w + 1.0 / w) / 2.0; // = cosh(sinh(j*h))
                    x = c - d * r;
                    y = f(x);
                    if y.is_finite() {
                        // if f(x) is finite, add to local sum
                        q += y * w;
                    }
                }

                x = c + d * r;
                y = f(x);
                if y.is_finite() {
                    // if f(x) is finite, add to local sum
                    q += y * w;
                }
                q *= t + 0.25 / t; // q *= cosh(j*h)
                p += q;
                t *= eh;
                if q.abs() <= eps * p.abs() {
                    break;
                }
            }
        }

        v = s - p;
        s += p;
        k += 1;
        if v.abs() <= tol * s.abs() || k > n {
            break;
        }
    }

    // if the estimated relative error is desired, then return it
    if let Some(err) = err {
        *err = v.abs() / (FUDGE2 * s.abs() + eps);
    }

    // result with estimated relative error err
    sign * d * s * h
}
