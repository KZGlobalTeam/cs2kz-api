//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};

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

	/// A request dedicated to a specific user was made, but the user could not
	/// be found.
	#[error("user does not exist")]
	UserDoesNotExist
	{
		/// The user's SteamID.
		user_id: SteamID,
	},

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
			Self::UserDoesNotExist { .. } => ProblemType::ResourceNotFound,
			Self::Database(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		if let Self::UserDoesNotExist { user_id } = self {
			ext.add("user_id", user_id);
		}
	}
}
