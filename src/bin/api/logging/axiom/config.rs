use std::fmt::{self, Debug};

use cs2kz_api::env::{self, Result};
use url::Url;

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

	/// URL for POSTing new logs.
	pub url: Url,
}

impl Config {
	pub fn new() -> Result<Self> {
		let token = env::get("AXIOM_TOKEN")?;
		let org_id = env::get("AXIOM_ORG_ID")?;
		let dataset = env::get("AXIOM_DATASET")?;
		let url = format!("https://api.axiom.co/v1/datasets/{dataset}/ingest")
			.parse()
			.expect("this is a valid url");

		Ok(Self { token, org_id, dataset, url })
	}
}

impl Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Config")
			.field("org_id", &self.org_id)
			.field("dataset", &self.dataset)
			.field("url", &self.url.as_str())
			.field("token", &"*****")
			.finish()
	}
}
