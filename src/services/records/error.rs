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

	/// A request targeted at a specific record was made, but the record could
	/// not be found.
	#[error("record does not exist")]
	RecordDoesNotExist,

	/// A request for moving a record from one status to another was made, but
	/// the requested status is the current status.
	#[error("cannot update record; supplied status is the same as current status")]
	WouldNotMove,

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
			Self::RecordDoesNotExist => ProblemType::ResourceNotFound,
			Self::WouldNotMove => ProblemType::NoChange,
			Self::Database(source) => source.problem_type(),
		}
	}
}
