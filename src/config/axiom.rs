use std::fmt;

use super::{get_env_var, Result};

/// Config for managing [axiom] log ingestions.
///
/// [axiom]: https://app.axiom.co
#[derive(Clone)]
pub struct Config {
	/// The access token used for authentication.
	pub token: String,

	/// The ID of the organization that is supposed to receive the logs.
	pub org_id: String,

	/// The specific dataset to add the logs to.
	pub dataset: String,

	/// A filter for [`tracing_subscriber`] logs.
	pub log_filter: String,
}

impl Config {
	pub fn new() -> Result<Self> {
		let token = get_env_var("AXIOM_TOKEN")?;
		let org_id = get_env_var("AXIOM_ORG_ID")?;
		let dataset = get_env_var("AXIOM_DATASET")?;
		let log_filter = get_env_var("AXIOM_LOG_FILTER")?;

		Ok(Self { token, org_id, dataset, log_filter })
	}
}

impl fmt::Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Axiom Config")
			.field("dataset", &self.dataset)
			.field("log_filter", &format_args!("{}", self.log_filter))
			.finish()
	}
}
