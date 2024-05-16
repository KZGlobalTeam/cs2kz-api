//! The main error type used across the code base.
//!
//! [`Error`] implements [`IntoResponse`], so that it can be returned by handlers.
//! Most fallible functions in this crate return [`Result<T>`].
//!
//! [`Error`]: struct@Error

use std::error::Error as StdError;
use std::fmt::Display;
use std::num::TryFromIntError;
use std::panic::Location;

use axum::extract::rejection::PathRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::typed_header::TypedHeaderRejection;
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error};

use crate::authorization::Permissions;
use crate::bans::UnbanID;
use crate::maps::{CourseID, FilterID, MapID};

/// Convenient type alias to use for fallible functions.
///
/// All fallible functions in this crate return an [`Error`] in their failure case, so spelling it
/// out 500 times is not desirable.
///
/// [`Error`]: struct@Error
pub type Result<T> = std::result::Result<T, Error>;

/// The main error type used in this crate.
///
/// Every fallible function returns it.
#[derive(Debug, Error)]
#[error("{}", message.as_deref().unwrap_or("something unexpected happened"))]
pub struct Error {
	/// The HTTP status code to use in the response.
	status: StatusCode,

	/// An error message to display to the user.
	message: Option<String>,

	/// Source code location of where the error occurred.
	location: &'static Location<'static>,

	/// An error source.
	source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Error {
	/// Create a new blank error with the given status code.
	#[track_caller]
	fn new(status: StatusCode) -> Self {
		Self {
			status,
			message: None,
			location: Location::caller(),
			source: None,
		}
	}

	/// Convenience function for logging out an error while creating it.
	fn with_loc<F>(self, f: F) -> Self
	where
		F: FnOnce(&Location<'_>),
	{
		f(self.location);
		self
	}

	/// Set the message of the error.
	fn with_message(mut self, message: impl Display) -> Self {
		self.message = Some(message.to_string());
		self
	}

	/// Set the source of the error.
	pub(crate) fn with_source<S>(mut self, source: S) -> Self
	where
		S: StdError + Send + Sync + 'static,
	{
		self.source = Some(Box::new(source));
		self
	}

	/// An unexpected error.
	#[track_caller]
	pub(crate) fn internal_server_error(message: impl Display) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR)
			.with_loc(|location| error!(target: "audit_log", %location, %message))
			.with_message(message)
	}

	/// Converting a database ID column type failed.
	#[track_caller]
	pub(crate) fn invalid_id_column(error: TryFromIntError) -> Self {
		Error::internal_server_error("invalid ID type").with_source(error)
	}

	/// `204 No Content` status code.
	#[track_caller]
	pub(crate) fn no_content() -> Self {
		Self::new(StatusCode::NO_CONTENT)
	}

	/// `401 Unauthorized` status code.
	#[track_caller]
	pub(crate) fn unauthorized() -> Self {
		Self::new(StatusCode::UNAUTHORIZED)
	}

	/// Some user input (e.g. an ID) is unknown / invalid.
	#[track_caller]
	pub(crate) fn unknown(what: impl Display) -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message(format_args!("unknown {what}"))
	}

	/// Some user input in a POST / PUT request already exists in the database.
	#[track_caller]
	pub(crate) fn already_exists(what: impl Display) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format_args!("{what} already exists"))
	}

	/// When PATCHing maps, the user shouldn't be allowed to remove all mappers from a map /
	/// course.
	#[track_caller]
	pub(crate) fn must_have_mappers() -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message("map/course cannot have 0 mappers")
	}

	/// When PATCHing maps, the user shouldn't be allowed to update courses that do not belong
	/// to the map.
	#[track_caller]
	pub(crate) fn course_does_not_belong_to_map(course_id: CourseID, map_id: MapID) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format_args!(
			"course with ID `{course_id}` does not belong to map `{map_id}`"
		))
	}

	/// When PATCHing maps, the user shouldn't be allowed to update filters that do not belong
	/// to courses on the map.
	#[track_caller]
	pub(crate) fn filter_does_not_belong_to_course(
		filter_id: FilterID,
		course_id: CourseID,
	) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format_args!(
			"filter with ID `{filter_id}` does not belong to course `{course_id}`"
		))
	}

	/// When submitting new plugin versions, the submitted version cannot be <= the current
	/// latest version.
	#[track_caller]
	pub(crate) fn invalid_semver(current_latest: &semver::Version) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format_args!(
			"invalid plugin version; current latest version is {current_latest}"
		))
	}

	/// When submitting new plugin versions, the submitted git revision must be unique.
	#[track_caller]
	pub(crate) fn invalid_plugin_rev() -> Self {
		Self::new(StatusCode::CONFLICT).with_message("this git revision is already in use")
	}

	/// When updating or deleting a ban, the ban might have already expired / reverted
	/// previously.
	#[track_caller]
	pub(crate) fn ban_already_reverted(unban_id: UnbanID) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format_args!(
			"ban has already been reverted (unban `{unban_id}`)"
		))
	}

	/// A CS2 server tried to request an access key (JWT) but their supplied refresh key was
	/// invalid.
	#[track_caller]
	pub(crate) fn invalid_refresh_key() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("refresh key is invalid")
	}

	/// A CS2 server made an authenticated request but their access key was expired.
	#[track_caller]
	pub(crate) fn expired_access_key() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("access key is expired")
	}

	/// A user tried to make an authenticated request but was missing their session ID.
	#[track_caller]
	pub(crate) fn missing_session_id() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("missing session ID")
	}

	/// A user tried to make an authenticated request but their session ID was invalid or
	/// expired.
	#[track_caller]
	pub(crate) fn invalid_session_id() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("invalid session ID")
	}

	/// A user tried to make an authenticated request but didn't have the required permissions.
	#[track_caller]
	pub(crate) fn insufficient_permissions(permissions: Permissions) -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message(format_args!(
			"you are missing the required roles to perform this action ({permissions})"
		))
	}

	/// A user tried to make an authenticated request but weren't the owner of the server they
	/// tried to PATCH.
	#[track_caller]
	pub(crate) fn must_be_server_owner() -> Self {
		Self::new(StatusCode::UNAUTHORIZED)
			.with_message("you must be the server's owner or an admin to perform this action")
	}

	/// An opaque API key was not a valid UUID.
	#[track_caller]
	pub(crate) fn key_must_be_uuid(error: uuid::Error) -> Self {
		Self::new(StatusCode::BAD_REQUEST)
			.with_message("key must be a valid UUID")
			.with_source(error)
	}

	/// An opaque API key was invalid.
	#[track_caller]
	pub(crate) fn key_invalid() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("key is invalid")
	}

	/// An opaque API key was expired.
	#[track_caller]
	pub(crate) fn key_expired() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("key has expired")
	}

	/// An external API call failed.
	#[track_caller]
	pub(crate) fn bad_gateway(message: impl Display) -> Self {
		Self::new(StatusCode::BAD_GATEWAY).with_message(message)
	}
}

impl IntoResponse for Error {
	#[allow(clippy::indexing_slicing)]
	fn into_response(self) -> Response {
		let Self {
			status,
			message,
			location,
			source,
		} = &self;

		debug!(%location, %status, ?message, ?source, "error occurred in request handler");

		let mut json = json!({ "message": self.to_string() });

		if let Some(source) = self
			.source
			.as_deref()
			.filter(|_| cfg!(not(feature = "production")))
		{
			json["debug_info"] = format!("{source:?}").into();
		}

		(self.status, Json(json)).into_response()
	}
}

impl From<sqlx::Error> for Error {
	#[track_caller]
	fn from(error: sqlx::Error) -> Self {
		use sqlx::Error as E;

		match error {
			E::Configuration(_) | E::Tls(_) | E::AnyDriverError(_) | E::Migrate(_) => {
				unreachable!("these do not happen after initial setup ({error})");
			}
			error => Self::internal_server_error("database error")
				.with_loc(|location| {
					error!(target: "audit_log", %error, %location, "database error");
				})
				.with_source(error),
		}
	}
}

impl From<jwt::errors::Error> for Error {
	#[track_caller]
	fn from(error: jwt::errors::Error) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR)
			.with_loc(|location| {
				error!(target: "audit_log", %error, %location, "failed to (de)serialize jwt");
			})
			.with_source(error)
	}
}

impl From<reqwest::Error> for Error {
	#[track_caller]
	fn from(error: reqwest::Error) -> Self {
		if matches!(error.status(), Some(status) if status.is_server_error()) {
			Self::new(StatusCode::BAD_GATEWAY)
		} else {
			Self::new(StatusCode::INTERNAL_SERVER_ERROR)
		}
		.with_message("failed to make http request")
		.with_source(error)
	}
}

impl From<TypedHeaderRejection> for Error {
	#[track_caller]
	fn from(rejection: TypedHeaderRejection) -> Self {
		Self::new(StatusCode::BAD_REQUEST)
			.with_message(rejection.to_string())
			.with_source(rejection)
	}
}

impl From<PathRejection> for Error {
	#[track_caller]
	fn from(rejection: PathRejection) -> Self {
		Self::new(rejection.status())
			.with_message(rejection.to_string())
			.with_source(rejection)
	}
}
