use std::convert::Infallible;
use std::result::Result as StdResult;

use thiserror::Error as ThisError;

pub type Result<T> = StdResult<T, Error>;

/// Any errors that can occurr while constructing the API's [Config].
///
/// [Config]: crate::config::Config
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum Error {
	#[error(transparent)]
	Environment(#[from] crate::env::Error),
}

impl From<Infallible> for Error {
	fn from(_: Infallible) -> Self {
		unreachable!()
	}
}
