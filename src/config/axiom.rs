use smart_debug::SmartDebug;
use url::Url;

use super::{get_env_var, Result};

/// Config for managing [axiom] log ingestions.
///
/// [axiom]: https://app.axiom.co
#[derive(SmartDebug, Clone)]
pub struct Config {
	/// The access token used for authentication.
	#[debug("…")]
	pub token: String,

	/// The ID of the organization that is supposed to receive the logs.
	#[debug("…")]
	pub org_id: String,

	/// The specific dataset to add the logs to.
	#[debug("…")]
	pub dataset: String,

	/// URL for POSTing new logs.
	pub url: Url,
}

impl Config {
	pub fn new() -> Result<Self> {
		let token = get_env_var("AXIOM_TOKEN")?;
		let org_id = get_env_var("AXIOM_ORG_ID")?;
		let dataset = get_env_var("AXIOM_DATASET")?;
		let url = format!("https://api.axiom.co/v1/datasets/{dataset}/ingest").parse()?;

		Ok(Self { token, org_id, dataset, url })
	}
}
