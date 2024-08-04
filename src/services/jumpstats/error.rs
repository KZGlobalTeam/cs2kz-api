//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use super::JumpstatID;
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

	/// A request dedicated to a specific player was made, but the player could
	/// not be found.
	#[error("player does not exist")]
	PlayerDoesNotExist
	{
		/// The player's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific jumpstat was made, but the player
	/// could not be found.
	#[error("jumpstat does not exist")]
	JumpstatDoesNotExist
	{
		/// The jumpstat's ID.
		jumpstat_id: JumpstatID,
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
			Self::PlayerDoesNotExist { .. } | Self::JumpstatDoesNotExist { .. } => {
				ProblemType::ResourceNotFound
			}
			Self::Database(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		match self {
			Self::PlayerDoesNotExist { steam_id } => {
				ext.add("steam_id", steam_id);
			}
			Self::JumpstatDoesNotExist { jumpstat_id } => {
				ext.add("jumpstat_id", jumpstat_id);
			}
			_ => {}
		}
	}
}
