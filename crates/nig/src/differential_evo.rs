use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Differential Evolution optimizer
///
/// Storn, R. and Price, K. Differential Evolution - A Simple and Efficient
/// Heuristic for Global Optimization over Continuous Spaces. Journal of
/// Global Optimization 11, 341-359 (1997).
pub fn differential_evolution<const N: usize>(
    objective: impl Fn(&[f64; N]) -> f64,
    bounds: &[(f64, f64); N],
    tol: f64,
    max_iter: usize,
    pop_factor: usize,
    mutation: (f64, f64),
    crossover: f64,
    seed: u64,
    inits: &[[f64; N]],
) -> ([f64; N], f64, usize) {
    const MAX_STAGNANT_GENERATIONS: usize = 25;

    let (f_lo, f_hi) = mutation;

    let mut rng = SmallRng::seed_from_u64(seed);
    let n = N;
    let np_pop = (pop_factor * n).max(inits.len() + 4);

    let lo: [f64; N] = core::array::from_fn(|i| bounds[i].0);
    let hi: [f64; N] = core::array::from_fn(|i| bounds[i].1);
    let span: [f64; N] = core::array::from_fn(|i| hi[i] - lo[i]);

    // Initialize population, seeding the first members with the provided
    // initial guesses (clamped to bounds).
    let mut pop: Vec<[f64; N]> = (0..np_pop)
        .map(|_| core::array::from_fn(|i| lo[i] + rng.r#gen::<f64>() * span[i]))
        .collect();
    for (member, init) in pop.iter_mut().zip(inits) {
        *member = core::array::from_fn(|i| init[i].clamp(lo[i], hi[i]));
    }
    let mut fitness: Vec<f64> = pop.iter().map(|p| objective(p)).collect();
    let mut nfev = np_pop;
    let mut stagnant_generations = 0usize;

    for _ in 0..max_iter {
        let mut improved = false;

        for i in 0..np_pop {
            // Pick three distinct random candidates (not i)
            let mut candidates: Vec<usize> = (0..np_pop).filter(|&j| j != i).collect();
            for k in 0..3 {
                let idx = rng.gen_range(k..candidates.len());
                candidates.swap(k, idx);
            }
            let a = candidates[0];
            let b = candidates[1];
            let c = candidates[2];

            let f = f_lo + rng.r#gen::<f64>() * (f_hi - f_lo);

            let mutant: [f64; N] = core::array::from_fn(|j| {
                (pop[a][j] + f * (pop[b][j] - pop[c][j])).clamp(lo[j], hi[j])
            });

            let mut cross_mask: [bool; N] =
                core::array::from_fn(|_| rng.r#gen::<f64>() < crossover);
            // ensure at least one dimension is taken from mutant
            cross_mask[rng.gen_range(0..n)] = true;

            let trial: [f64; N] =
                core::array::from_fn(|j| if cross_mask[j] { mutant[j] } else { pop[i][j] });

            let f_trial = objective(&trial);
            nfev += 1;

            if f_trial <= fitness[i] {
                pop[i] = trial;
                fitness[i] = f_trial;
                improved = true;
            }
        }

        let best_idx = argmin(&fitness);
        if improved {
            stagnant_generations = 0;
        } else {
            stagnant_generations += 1;
            if stagnant_generations >= MAX_STAGNANT_GENERATIONS {
                break;
            }
        }

        // Population spread check
        let spread = (fitness
            .iter()
            .fold(0.0_f64, |acc, &fv| (fv - fitness[best_idx]).abs().max(acc)))
            / (1.0 + fitness[best_idx].abs());
        if spread <= tol {
            break;
        }
    }

    let best_idx = argmin(&fitness);
    (pop[best_idx], fitness[best_idx], nfev)
}

fn argmin(slice: &[f64]) -> usize {
    slice
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .unwrap_or(0)
}
