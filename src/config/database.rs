use std::fmt::{self, Debug};

use url::Url;

use crate::env::{self, Result};

/// Configuration for managing database connections.
pub struct Config {
	/// Database URL to connect to.
	pub url: Url,
}

impl Config {
	pub fn new() -> Result<Self> {
		let url = env::get("DATABASE_URL")?;

		Ok(Self { url })
	}
}

impl Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Config")
			.field("url", &self.url.as_str())
			.finish()
	}
}
