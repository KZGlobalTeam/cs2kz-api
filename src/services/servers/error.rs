//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::services::auth;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the server service.
#[derive(Debug, Error)]
pub enum Error
{
	/// We have no data to return.
	#[error("no data")]
	NoData,

	/// A request involving a specific server owner was made, but the player
	/// could not be found.
	#[error("server owner does not exist")]
	ServerOwnerDoesNotExist
	{
		/// The server owner's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific server was made, but the server could
	/// not be found.
	#[error("server does not exist")]
	ServerDoesNotExist,

	/// A request containing an API key and plugin version was made, but one of
	/// them was invalid.
	#[error("invalid key or plugin version")]
	InvalidKeyOrPluginVersion,

	/// Something went wrong when interacting with the auth service.
	#[error(transparent)]
	Auth(#[from] auth::Error),

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
			Self::ServerOwnerDoesNotExist { .. } | Self::ServerDoesNotExist => {
				ProblemType::ResourceNotFound
			}
			Self::InvalidKeyOrPluginVersion => ProblemType::Unauthorized,
			Self::Auth(source) => source.problem_type(),
			Self::Database(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		match self {
			Self::ServerOwnerDoesNotExist { steam_id } => {
				ext.add("owner_id", steam_id);
			}
			Self::Auth(source) => {
				source.add_extension_members(ext);
			}
			_ => {}
		}
	}
}
