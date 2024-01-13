use std::fmt;

use super::{get_env_var, Result};

/// Configuration for communicating with Steam.
pub struct Config {
	/// Steam [WebAPI] Key.
	///
	/// [WebAPI]: https://steamcommunity.com/dev
	pub api_key: String,
}

impl Config {
	pub fn new() -> Result<Self> {
		let api_key = get_env_var("KZ_API_STEAM_API_KEY")?;

		Ok(Self { api_key })
	}
}

impl fmt::Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Steam Config")
			.field("api_key", &"…")
			.finish()
	}
}
