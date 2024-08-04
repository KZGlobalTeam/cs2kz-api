//! The errors that can occur when interacting with this service.

use thiserror::Error;

use super::PluginVersion;
use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// A request targeted at a specific plugin version was made, but the plugin
	/// version could not be found.
	#[error("plugin version does not exist")]
	VersionDoesNotExist,

	/// A plugin version was submitted, but its semver version is older than the
	/// latest version that is in the database.
	#[error("submitted version is older than the latest version")]
	OutdatedVersion
	{
		/// The current latest version.
		latest: PluginVersion,

		/// The submitted version.
		actual: PluginVersion,
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
			Self::VersionDoesNotExist => ProblemType::ResourceNotFound,
			Self::OutdatedVersion { .. } => ProblemType::OutdatedVersion,
			Self::Database(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		if let Self::OutdatedVersion { latest, actual } = self {
			ext.add("latest_version", latest);
			ext.add("actual_version", actual);
		}
	}
}
