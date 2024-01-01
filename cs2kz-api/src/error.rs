//! This module contains the main error type for the API.
//!
//! Any runtime errors that are expected to happen are defined in here.

use std::error::Error as StdError;
use std::result::Result as StdResult;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use cs2kz::SteamID;
use serde_json::json;
use thiserror::Error;
use tracing::error;

/// Convenience type alias for the crate's main error type.
pub type Result<T> = StdResult<T, Error>;

/// The main error type of this crate.
///
/// These errors might occurr during runtime.
#[derive(Debug, Error)]
pub enum Error {
	/// Something unexpected happened.
	///
	/// This is a catch-all error type and will always result in a 500 when returned from
	/// an HTTP handler.
	#[error("Something unexpected happened. This is a bug.")]
	Unexpected(Box<dyn StdError>),

	/// A database query returned 0 rows.
	#[error("No data available for the given query.")]
	NoContent,

	/// Request body could not be parsed.
	#[error("Invalid request body. Expected bytes.")]
	InvalidRequestBody,

	/// A request had missing / invalid credentials.
	///
	/// This error usually occurrs in authentication middelware.
	#[error("You do not have the required permissions to access this resource.")]
	Unauthorized,

	/// A request for creating a new map was missing a course for a specific stage.
	#[error("Missing course for stage `{stage}`.")]
	MissingCourse {
		/// The stage in question.
		stage: u8,
	},

	/// A request for creating a record had an invalid (course, mode, teleports)
	/// combination.
	#[error("The submitted record does not have a filter.")]
	InvalidFilter,

	/// A request had a body with a SteamID which does not exist in the database.
	#[error("Unknown Player with SteamID `{steam_id}`.")]
	UnknownPlayer {
		/// The player's SteamID.
		steam_id: SteamID,
	},
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		let message = self.to_string();
		let code = match self {
			Self::Unexpected(err) => {
				error!(error = ?err, "Unexpected error happened");

				StatusCode::INTERNAL_SERVER_ERROR
			}

			Self::NoContent => StatusCode::NO_CONTENT,
			Self::InvalidRequestBody | Self::UnknownPlayer { .. } => StatusCode::BAD_REQUEST,
			Self::MissingCourse { .. } | Self::InvalidFilter => StatusCode::CONFLICT,
			Self::Unauthorized => StatusCode::UNAUTHORIZED,
		};

		(code, Json(json!({ "message": message }))).into_response()
	}
}

impl From<sqlx::Error> for Error {
	fn from(error: sqlx::Error) -> Self {
		use sqlx::Error as E;

		#[allow(clippy::wildcard_in_or_patterns)]
		match error {
			E::RowNotFound => Self::NoContent,

			E::Database(_)
			| E::PoolTimedOut
			| E::PoolClosed
			| E::WorkerCrashed
			| E::AnyDriverError(_)
			| E::Migrate(_) => panic!("Fatal database error: {error}"),

			E::Configuration(_)
			| E::Io(_)
			| E::Tls(_)
			| E::Protocol(_)
			| E::TypeNotFound { .. }
			| E::ColumnIndexOutOfBounds { .. }
			| E::ColumnNotFound(_)
			| E::ColumnDecode { .. }
			| E::Decode(_)
			| _ => Self::Unexpected(Box::new(error)),
		}
	}
}

impl From<jwt::errors::Error> for Error {
	fn from(error: jwt::errors::Error) -> Self {
		use jwt::errors::ErrorKind as E;

		#[allow(clippy::wildcard_in_or_patterns)]
		match error.kind() {
			E::InvalidToken
			| E::InvalidSignature
			| E::MissingRequiredClaim(_)
			| E::ExpiredSignature
			| E::InvalidIssuer
			| E::InvalidAudience
			| E::InvalidSubject
			| E::ImmatureSignature
			| E::InvalidAlgorithm
			| E::MissingAlgorithm => Self::Unauthorized,

			E::Base64(_)
			| E::Json(_)
			| E::Utf8(_)
			| E::Crypto(_)
			| E::InvalidEcdsaKey
			| E::InvalidRsaKey(_)
			| E::RsaFailedSigning
			| E::InvalidAlgorithmName
			| E::InvalidKeyFormat
			| _ => Self::Unexpected(Box::new(error)),
		}
	}
}
