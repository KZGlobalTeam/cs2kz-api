use std::fmt::{self, Debug};

use crate::env::{self, Result};

/// Configuration for managing [JWTs].
///
/// [JWTs]: https://jwt.io
pub struct Config {
	/// The secret used for encoding / decoding payloads.
	pub secret: String,
}

impl Config {
	pub fn new() -> Result<Self> {
		let secret = env::get("API_JWT_SECRET")?;

		Ok(Self { secret })
	}
}

impl Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Config").field("secret", &"*****").finish()
	}
}
