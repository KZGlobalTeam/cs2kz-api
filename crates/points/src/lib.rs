use pyo3::{PyResult, Python};

mod record;
pub use record::Record;

mod dist;
pub use dist::NormInvGauss;

pub fn calculate_points(py: Python<'_>, records: &[Record]) -> PyResult<Vec<u16>> {
	let times = records.iter().map(|record| record.time).collect::<Vec<_>>();
	let norminvgauss = NormInvGauss::new(py, &times)?;
	let tier = records[0].stage_tier;
	let pro_only = records.iter().all(|record| record.teleports == 0);
	let points = records
		.iter()
		.enumerate()
		.map(|(rank, record)| norminvgauss.calc_points(record, rank, tier, pro_only))
		.collect::<PyResult<Vec<_>>>()?;

	Ok(points)
}
