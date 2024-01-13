use std::fmt;

use url::Url;

use super::{get_env_var, Result};

/// Configuration for managing database connections.
pub struct Config {
	/// Database URL to connect to.
	pub url: Url,
}

impl Config {
	pub fn new() -> Result<Self> {
		let url = get_env_var("DATABASE_URL")?;

		Ok(Self { url })
	}
}

impl fmt::Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("MySQL Config")
			.field("url", &format_args!("{}", self.url))
			.finish()
	}
}
