/// Nelder-Mead downhill simplex minimizer, used to polish the result of the
/// differential evolution global search.
///
/// Nelder, J. A. and Mead, R. A Simplex Method for Function Minimization.
/// The Computer Journal 7, 308-313 (1965).
pub fn nelder_mead<const N: usize>(
    objective: impl Fn(&[f64; N]) -> f64,
    start: &[f64; N],
    max_iter: usize,
    tol: f64,
) -> ([f64; N], f64, usize) {
    const ALPHA: f64 = 1.0; // reflection
    const GAMMA: f64 = 2.0; // expansion
    const RHO: f64 = 0.5; // contraction
    const SIGMA: f64 = 0.5; // shrink

    // Initial simplex: perturb each coordinate (scipy-style).
    let mut simplex: Vec<[f64; N]> = Vec::with_capacity(N + 1);
    simplex.push(*start);
    for i in 0..N {
        let mut vertex = *start;
        if vertex[i] != 0.0 {
            vertex[i] *= 1.05;
        } else {
            vertex[i] = 0.00025;
        }
        simplex.push(vertex);
    }

    let mut values: Vec<f64> = simplex.iter().map(&objective).collect();
    let mut nfev = N + 1;

    for _ in 0..max_iter {
        // Sort vertices by objective value.
        let mut order: Vec<usize> = (0..=N).collect();
        order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
        let simplex_sorted: Vec<[f64; N]> = order.iter().map(|&i| simplex[i]).collect();
        let values_sorted: Vec<f64> = order.iter().map(|&i| values[i]).collect();
        simplex = simplex_sorted;
        values = values_sorted;

        let best = values[0];
        let worst = values[N];
        if (worst - best).abs() <= tol * (1.0 + best.abs()) {
            break;
        }

        // Centroid of all vertices except the worst.
        let centroid: [f64; N] =
            core::array::from_fn(|j| simplex[..N].iter().map(|v| v[j]).sum::<f64>() / N as f64);

        let reflected: [f64; N] =
            core::array::from_fn(|j| centroid[j] + ALPHA * (centroid[j] - simplex[N][j]));
        let f_reflected = objective(&reflected);
        nfev += 1;

        if f_reflected < values[0] {
            // Try expanding further in the same direction.
            let expanded: [f64; N] =
                core::array::from_fn(|j| centroid[j] + GAMMA * (reflected[j] - centroid[j]));
            let f_expanded = objective(&expanded);
            nfev += 1;

            if f_expanded < f_reflected {
                simplex[N] = expanded;
                values[N] = f_expanded;
            } else {
                simplex[N] = reflected;
                values[N] = f_reflected;
            }
        } else if f_reflected < values[N - 1] {
            simplex[N] = reflected;
            values[N] = f_reflected;
        } else {
            // Contract towards the centroid.
            let contracted: [f64; N] =
                core::array::from_fn(|j| centroid[j] + RHO * (simplex[N][j] - centroid[j]));
            let f_contracted = objective(&contracted);
            nfev += 1;

            if f_contracted < values[N] {
                simplex[N] = contracted;
                values[N] = f_contracted;
            } else {
                // Shrink the whole simplex towards the best vertex.
                for i in 1..=N {
                    simplex[i] = core::array::from_fn(|j| {
                        simplex[0][j] + SIGMA * (simplex[i][j] - simplex[0][j])
                    });
                    values[i] = objective(&simplex[i]);
                }
                nfev += N;
            }
        }
    }

    let mut best_idx = 0;
    for i in 1..=N {
        if values[i] < values[best_idx] {
            best_idx = i;
        }
    }

    (simplex[best_idx], values[best_idx], nfev)
}
