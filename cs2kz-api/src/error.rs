//! This module contains the main error type for the API.
//!
//! Any runtime errors that are expected to happen are defined in here.

use std::error::Error as StdError;
use std::result::Result as StdResult;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use cs2kz::{Mode, SteamID, Tier};
use serde_json::{json, Value as JsonValue};
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
	Unexpected(Box<dyn StdError + Send>),

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

	/// A server submitted a jumpstat that wasn't a player's PB.
	#[error("The submitted jumpstat is not a personal best.")]
	NotPersonalBest,

	/// A submitted map was missing a required field (empty arrays count as missing fields).
	#[error("Missing required field `{0}`.")]
	MissingMapField(&'static str),

	/// A submitted map was missing a particular filter.
	#[error("Missing ({mode}, {runtype}) filter on stage {stage}.", runtype = match teleports {
		true => "TP",
		false => "PRO",
	})]
	MissingFilter {
		/// The stage this course belongs to.
		stage: u8,

		/// The mode this filter counts for.
		mode: Mode,

		/// Whether this filter counts for runs with teleports.
		teleports: bool,
	},

	/// A submitted filter's tier was too high for it to be ranked.
	#[error("T{} is too high to be ranked.", *tier as u8)]
	TooDifficultToRank {
		/// The tier that is too high for this filter to be ranked.
		tier: Tier,
	},

	/// A submitted map has an invalid Steam Workshop ID.
	#[error("Workshop ID `{0}` is not a valid ID.")]
	InvalidWorkshopID(u32),

	/// A course update specified an unknown pair of map & stage.
	#[error("A course with stage `{stage}` on the map with ID `{map_id}` does not exist.")]
	InvalidMapOrStage {
		/// The ID of the map the course belongs to.
		map_id: u16,

		/// The stage the course is associated with.
		stage: u8,
	},

	/// A request for creating a new map requested a map to be created which already
	/// exists.
	#[error("This map already exists.")]
	MapExists,

	/// A request contained a map ID that is not in the database.
	#[error("Unknown Map ID `{0}`.")]
	UnknownMapID(u16),

	/// A map update wanted to global a map, although another version of that map is still
	/// global.
	#[error("Another version of this map is still global. Please deglobal map `{id}` first.")]
	MapAlreadyGlobal {
		/// The ID of the map with the same name that is already global.
		id: u16,
	},

	/// A course update for a course that does not match the map to be updated.
	#[error("The course with ID `{id}` is not part of map `{map_id}`.")]
	MismatchingCourse {
		/// The ID of the course.
		id: u32,

		/// The ID of the map.
		map_id: u16,
	},

	/// A filter update contained a filter ID that was not part of the course it was
	/// supposed to affect.
	#[error("Filter with ID `{id}` is not part of course `{course_id}`.")]
	MismatchingFilter {
		/// The ID of the filter.
		id: u32,

		/// The ID of the course that was submitted, but didn't match.
		course_id: u32,
	},

	/// Something went wrong making a request to the Steam API.
	#[error("Steam API error.")]
	SteamAPI(reqwest::Error),

	/// Something went wrong downloading a map from the Steam Workshop.
	#[error("Failed to download map from Workshop.")]
	WorkshopMapDownload,
}

impl IntoResponse for Error {
	fn into_response(self) -> axum::response::Response {
		let mut body = json!({ "message": self.to_string() });
		let code = match self {
			Self::Unexpected(error) => {
				error!(audit = true, ?error, "Unexpected error happened");

				StatusCode::INTERNAL_SERVER_ERROR
			}

			Self::NoContent => StatusCode::NO_CONTENT,
			Self::InvalidRequestBody
			| Self::UnknownPlayer { .. }
			| Self::MissingMapField(_)
			| Self::MissingFilter { .. }
			| Self::TooDifficultToRank { .. }
			| Self::InvalidWorkshopID(_)
			| Self::UnknownMapID(_)
			| Self::MapAlreadyGlobal { .. } => StatusCode::BAD_REQUEST,
			Self::InvalidFilter
			| Self::NotPersonalBest
			| Self::InvalidMapOrStage { .. }
			| Self::MapExists
			| Self::MismatchingCourse { .. }
			| Self::MismatchingFilter { .. } => StatusCode::CONFLICT,
			Self::Unauthorized => StatusCode::UNAUTHORIZED,
			Self::SteamAPI(error) => {
				body["error"] = JsonValue::String(error.to_string());
				StatusCode::BAD_GATEWAY
			}
			Self::WorkshopMapDownload => StatusCode::INTERNAL_SERVER_ERROR,
		};

		(code, Json(body)).into_response()
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
