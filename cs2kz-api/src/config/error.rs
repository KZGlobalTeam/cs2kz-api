use std::result::Result as StdResult;

use thiserror::Error;

/// Convenience type alias.
pub type Result<T> = StdResult<T, Error>;

/// Errors that can occurr during API setup.
#[derive(Debug, Error)]
pub enum Error {
	/// A necessary environment variable was not set.
	#[error("Missing configuration environment variable `{variable}`.")]
	MissingConfigVariable {
		/// The variable in question.
		variable: &'static str,
	},

	/// A necessary environment variable was set, but had an invalid type.
	#[error("Invalid configuration environment variable `{variable}`. Expected `{expected}`.")]
	InvalidConfigVariable {
		/// The variable in question.
		variable: &'static str,

		/// The expected type of the variable.
		expected: &'static str,
	},
}
