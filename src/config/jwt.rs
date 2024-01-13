use std::fmt;

use super::{get_env_var, Result};

/// Configuration for managing [JWTs].
///
/// [JWTs]: https://jwt.io
pub struct Config {
	/// The secret used for encoding / decoding payloads.
	pub secret: String,
}

impl Config {
	pub fn new() -> Result<Self> {
		let secret = get_env_var("KZ_API_JWT_SECRET")?;

		Ok(Self { secret })
	}
}

impl fmt::Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("JWT Config").field("secret", &"…").finish()
	}
}
