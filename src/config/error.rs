use core::num;
use std::convert::Infallible;
use std::net;
use std::result::Result as StdResult;

use thiserror::Error as ThisError;

use super::environment::InvalidEnvironment;

pub type Result<T> = StdResult<T, Error>;

/// Any errors that can occurr while constructing the API's [Config].
///
/// [Config]: crate::config::Config
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum Error {
	#[error("Missing environment variable `{0}`.")]
	MissingEnvironmentVariable(&'static str),

	#[error("Failed to parse network address: {0}")]
	InvalidSocketAddr(#[from] net::AddrParseError),

	#[error("Failed to parse port number: {0}")]
	InvalidPort(#[from] num::ParseIntError),

	#[error("Failed to parse URL: {0}")]
	InvalidURL(#[from] url::ParseError),

	#[error("Failed to parse runtime environment: {0}")]
	InvalidRuntimeEnvironment(#[from] InvalidEnvironment),

	#[error("Failed to parse log filter: {0}")]
	InvalidLogFilter(#[from] tracing_subscriber::filter::ParseError),

	#[doc(hidden)]
	#[error("")]
	Never,
}

impl From<Infallible> for Error {
	fn from(_: Infallible) -> Self {
		Self::Never
	}
}
