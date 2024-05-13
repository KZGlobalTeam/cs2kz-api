use cs2kz::Tier;
use pyo3::types::{PyAny, PyAnyMethods, PyList, PyTuple};
use pyo3::{PyResult, Python};

use crate::Record;

#[derive(Debug)]
pub struct NormInvGauss<'py> {
	total: usize,
	max: f64,
	cdf: pyo3::Bound<'py, PyAny>,
	sf: f64,
}

impl<'py> NormInvGauss<'py> {
	pub fn new(py: Python<'py>, stats: &[f64]) -> PyResult<Self> {
		let total = stats.len();
		let max = stats[0];
		let norminvgauss = py.import_bound("scipy.stats")?.getattr("norminvgauss")?;

		let parameters = norminvgauss
			.getattr("fit")?
			.call1((PyList::new_bound(py, stats),))?
			.downcast_into::<PyTuple>()?;

		let cdf = norminvgauss.call1(parameters)?.getattr("cdf")?;
		let sf = 1.0 - cdf.call1((max,))?.extract::<f64>()?;

		Ok(Self { total, max, cdf, sf })
	}

	pub fn calc_points(
		&self,
		record: &Record,
		rank: usize,
		tier: Tier,
		pro_only: bool,
	) -> PyResult<u16> {
		let mut minimum_points = match tier {
			Tier::VeryEasy => 0.0,
			Tier::Easy => 500.0,
			Tier::Medium => 2000.0,
			Tier::Advanced => 3500.0,
			Tier::Hard => 5000.0,
			Tier::VeryHard => 6500.0,
			Tier::Extreme => 8000.0,
			Tier::Death => 9500.0,
			Tier::Unfeasible | Tier::Impossible => unreachable!(),
		};

		let remaining_points = 10000.0 - minimum_points;

		if pro_only {
			minimum_points += remaining_points * 0.1;
		}

		let rank_points = remaining_points * points_for_rank(rank, self.total) * 0.25;
		let dist_points = remaining_points
			* 0.75 * if self.total < 50 {
			low_completion_points(record.time, self.max, tier)
		} else {
			(1.0 - self.cdf.call1((record.time,))?.extract::<f64>()?) / self.sf
		};

		Ok((minimum_points + dist_points + rank_points) as u16)
	}
}

fn points_for_rank(rank: usize, total: usize) -> f64 {
	let mut points = 0.5 * (1.0 - rank as f64 / total as f64);

	if rank < 100 {
		points += (100 - rank) as f64 * 0.002;
	}

	if rank < 20 {
		points += (20 - rank) as f64 * 0.01;
	}

	match rank {
		0 => points += 0.1,
		1 => points += 0.06,
		2 => points += 0.045,
		3 => points += 0.03,
		4 => points += 0.01,
		_ => {}
	}

	points
}

fn low_completion_points(time: f64, wr_time: f64, tier: Tier) -> f64 {
	let tier = u8::from(tier) as f64;
	let x = 2.1 - 0.25 * tier;
	let y = 1.0 + (x * -0.5).exp();
	let z = 1.0 + (x * (time / wr_time - 1.5)).exp();

	y / z
}
