//! Rejection types for [`Session`].
//!
//! [`Session`]: super::Session

use std::str::FromStr;

use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::http::problem_details::{IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;
use crate::services::auth::SessionID;

/// Error that can occur while authenticating a session.
#[derive(Debug, Error)]
pub enum SessionRejection
{
	/// The cookie holding the session ID is missing.
	#[error("missing session cookie")]
	MissingCookie,

	/// The session ID could not be parsed.
	#[error("could not parse session ID: {source}")]
	ParseSessionID
	{
		/// The original error we got from `value.parse()`.
		source: <SessionID as FromStr>::Err,
	},

	/// The session ID was invalid.
	///
	/// This happens either because the ID is not in the database, or the
	/// session associated with that ID already expired.
	#[error("invalid session id")]
	InvalidSessionID,

	/// Something went wrong communicating with the database.
	#[error("something went wrong")]
	Database(#[from] sqlx::Error),
}

impl IntoProblemDetails for SessionRejection
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::MissingCookie => ProblemType::MissingHeader,
			Self::ParseSessionID { .. } => ProblemType::InvalidHeader,
			Self::InvalidSessionID => ProblemType::Unauthorized,
			Self::Database(source) => source.problem_type(),
		}
	}
}

impl IntoResponse for SessionRejection
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}
