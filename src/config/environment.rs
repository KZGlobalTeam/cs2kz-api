use std::str::FromStr;

use thiserror::Error as ThisError;

/// A runtime environment.
///
/// This can be used to branch the API's behavior based on where it is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
	/// The API is running locally on a developer's machine.
	Local,

	/// The API is running in prodction.
	Production,
}

impl Environment {
	pub const fn is_dev(&self) -> bool {
		matches!(self, Self::Local)
	}

	pub const fn is_prod(&self) -> bool {
		matches!(self, Self::Production)
	}
}

#[derive(Debug, ThisError)]
#[error("`{0}` is not a valid runtime environment. Expected `local` or `production`.")]
pub struct InvalidEnvironment(String);

impl FromStr for Environment {
	type Err = InvalidEnvironment;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		match input {
			"local" => Ok(Self::Local),
			"production" => Ok(Self::Production),
			invalid => Err(InvalidEnvironment(invalid.to_owned())),
		}
	}
}
