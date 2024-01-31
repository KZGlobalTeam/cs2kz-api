use std::result::Result as StdResult;

use thiserror::Error as ThisError;

pub type Result<T> = StdResult<T, Error>;

/// Any errors that can occurr when creating or using the API's [State].
///
/// [State]: crate::State
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, ThisError)]
pub enum Error {
	#[error("Error connecting to MySQL: {0}")]
	MySQL(#[from] sqlx::Error),

	#[error("Error encoding JSON: {0}")]
	JsonEncode(serde_json::Error),

	#[error("Error decoding JSON: {0}")]
	JsonDecode(serde_json::Error),

	#[error("Error generating JWT: {0}")]
	Jwt(#[from] jsonwebtoken::errors::Error),

	#[error("Error encoding JWT: {0}")]
	JwtEncode(jsonwebtoken::errors::Error),

	#[error("Error decoding JWT: {0}")]
	JwtDecode(jsonwebtoken::errors::Error),
}
