use std::result::Result as StdResult;

use thiserror::Error;

use crate::config;

/// Convenience type alias.
pub type Result<T> = StdResult<T, Error>;

/// Errors that can occurr during API setup.
#[derive(Debug, Error)]
pub enum Error {
	/// There was an error trying to connect to the API's database.
	#[error("Failed to establish database connection: {0}")]
	DatabaseConnection(#[from] sqlx::Error),

	/// There was an error trying to load and parse the API's JWT secret.
	#[error("Failed to load JWT data: {0}")]
	JWT(#[from] jwt::errors::Error),

	/// Something went wrong with the API configuration.
	#[error("API configuration error: {0}")]
	Config(#[from] config::Error),

	/// Something went wrong with data serialization.
	#[error("Failed to serialize data: {0}")]
	Serialize(#[from] serde_urlencoded::ser::Error),

	/// Something went wrong trying to parse a URL.
	#[error("Failed to parse URL: {0}")]
	Url(#[from] url::ParseError),
}
