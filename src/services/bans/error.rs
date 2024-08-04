//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use super::{BanID, UnbanID};
use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::services::servers::ServerID;

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

	/// A player who is already banned, cannot be banned again.
	#[error("player is already banned")]
	PlayerAlreadyBanned
	{
		/// The player's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific player was made, but the player could
	/// not be found.
	#[error("player does not exist")]
	PlayerDoesNotExist
	{
		/// The player's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific ban was made, but the ban could
	/// not be found.
	#[error("ban does not exist")]
	BanDoesNotExist
	{
		/// The ban's ID.
		ban_id: BanID,
	},

	/// A ban update requested the ban's expiration date to be set to a date
	/// before the ban's creation.
	#[error("ban cannot expire before it was created")]
	ExpirationBeforeCreation,

	/// A request was made to update an already reverted ban.
	#[error("ban has already been reverted")]
	BanAlreadyReverted
	{
		/// The unban's ID.
		unban_id: UnbanID,
	},

	/// A request for submitting a ban was rejected due to lack of
	/// authorization.
	#[error("you are not authorized to perform this action")]
	Unauthorized,

	/// A request for submitting a ban was rejected because it was authenticated
	/// as both a CS2 server and a session.
	#[error("you are not authorized to perform this action")]
	DoublyAuthorized
	{
		/// The server ID found in the JWT.
		server_id: ServerID,

		/// The user ID found in the session.
		admin_id: SteamID,
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
			Self::PlayerAlreadyBanned { .. } | Self::BanAlreadyReverted { .. } => {
				ProblemType::ActionAlreadyPerformed
			}
			Self::ExpirationBeforeCreation => ProblemType::IllogicalTimestamp,
			Self::PlayerDoesNotExist { .. } | Self::BanDoesNotExist { .. } => {
				ProblemType::ResourceNotFound
			}
			Self::Unauthorized | Self::DoublyAuthorized { .. } => ProblemType::Unauthorized,
			Self::Database(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		match self {
			Self::PlayerAlreadyBanned { steam_id } | Self::PlayerDoesNotExist { steam_id } => {
				ext.add("steam_id", steam_id);
			}
			Self::BanDoesNotExist { ban_id } => {
				ext.add("ban_id", ban_id);
			}
			Self::BanAlreadyReverted { unban_id } => {
				ext.add("unban_id", unban_id);
			}
			_ => {}
		}
	}
}
