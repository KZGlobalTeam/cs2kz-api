//! The errors that can occur when interacting with this service.

use thiserror::Error;

use crate::http::problem_details::{IntoProblemDetails, ProblemType};

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// We have no data to return.
	#[error("no data")]
	NoData,

	/// A request for registering a new player was made, but the player already
	/// existed in the database.
	#[error("player already exists")]
	PlayerAlreadyExists,

	/// A request targeted at a specific player was made, but the player could
	/// not be found in the database.
	#[error("player does not exist")]
	PlayerDoesNotExist,

	/// Something went wrong communicating with the database.
	#[error("something went wrong")]
	Database(#[from] sqlx::Error),
}

impl IntoProblemDetails for Error
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::NoData => ProblemType::NoContent,
			Self::PlayerAlreadyExists => ProblemType::ResourceAlreadyExists,
			Self::PlayerDoesNotExist => ProblemType::ResourceNotFound,
			Self::Database(source) => source.problem_type(),
		}
	}
}
