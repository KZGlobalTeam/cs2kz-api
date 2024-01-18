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

	#[error("Error encoding / decoding JSON: {0}")]
	Json(#[from] serde_json::Error),

	#[error("Error generating JWT: {0}")]
	JWT(#[from] jsonwebtoken::errors::Error),
}
