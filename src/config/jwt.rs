use smart_debug::SmartDebug;

use super::{get_env_var, Result};

/// Configuration for managing [JWTs].
///
/// [JWTs]: https://jwt.io
#[derive(SmartDebug)]
pub struct Config {
	/// The secret used for encoding / decoding payloads.
	#[debug("…")]
	pub secret: String,
}

impl Config {
	pub fn new() -> Result<Self> {
		let secret = get_env_var("KZ_API_JWT_SECRET")?;

		Ok(Self { secret })
	}
}
