//! The errors that can occur when interacting with this service.

use thiserror::Error;

use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::services::steam;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the auth service.
#[derive(Debug, Error)]
pub enum Error
{
	/// We failed to encode a JWT.
	///
	/// This should realistically never happen, as our data structures are known
	/// statically, and we only encode raw strings or JSON. If we ever happen to
	/// try and encode a particular value into JSON that can't be valid JSON
	/// however, we will fail.
	#[error("failed to encode jwt")]
	EncodeJwt
	{
		/// The original error we got from the JWT library.
		source: jsonwebtoken::errors::Error,
	},

	/// We failed to decode a JWT.
	#[error("failed to decode jwt: {source}")]
	DecodeJwt
	{
		/// The original error we got from the JWT library.
		source: jsonwebtoken::errors::Error,
	},

	/// An operation using the steam service failed.
	#[error(transparent)]
	Steam(#[from] steam::Error),

	/// Something went wrong communicating with the database.
	#[error("something went wrong")]
	Database(#[from] sqlx::Error),
}

impl IntoProblemDetails for Error
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::EncodeJwt { .. } => ProblemType::Internal,
			Self::Database(source) => source.problem_type(),
			Self::DecodeJwt { .. } => ProblemType::InvalidHeader,
			Self::Steam(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		if let Self::Steam(source) = self {
			source.add_extension_members(ext);
		}
	}
}
