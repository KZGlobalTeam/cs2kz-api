use smart_debug::SmartDebug;
use url::Url;

use super::{get_env_var, Result};

/// Configuration for managing database connections.
#[derive(SmartDebug)]
pub struct Config {
	/// Database URL to connect to.
	#[debug("…")]
	pub url: Url,
}

impl Config {
	pub fn new() -> Result<Self> {
		let url = get_env_var("DATABASE_URL")?;

		Ok(Self { url })
	}
}
