use std::fmt::Debug;
use std::result::Result as StdResult;
use std::str::FromStr;

use thiserror::Error as ThisError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
	#[error("missing environment variable `{var}`")]
	Missing { var: &'static str },

	#[error("failed to parse environment variable `{var}`: {message}")]
	Parse { var: &'static str, message: String },
}

pub fn get<T>(var: &'static str) -> Result<T>
where
	T: FromStr,
	<T as FromStr>::Err: std::error::Error,
{
	std::env::var(var)
		.map_err(|_| Error::Missing { var })?
		.parse::<T>()
		.map_err(|err| Error::Parse { var, message: err.to_string() })
}
