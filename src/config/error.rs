use core::num;
use std::convert::Infallible;
use std::net;
use std::result::Result as StdResult;

use thiserror::Error as ThisError;

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

	#[error("Failed to parse log filter: {0}")]
	InvalidLogFilter(#[from] tracing_subscriber::filter::ParseError),
}

impl From<Infallible> for Error {
	fn from(_: Infallible) -> Self {
		unreachable!()
	}
}
