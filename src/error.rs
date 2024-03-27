//! The main error type used across the code base.
//!
//! [`Error`] implements [`IntoResponse`], so that it can be returned by handlers.
//! Most fallible functions in this crate return [`Result<T>`].
//!
//! [`Error`]: struct@Error

use std::error::Error as StdError;
use std::fmt::Display;
use std::io;
use std::panic::Location;

use axum::extract::rejection::PathRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::typed_header::TypedHeaderRejection;
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error};

use crate::auth::RoleFlags;

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
#[error("{}", match message.as_deref() {
	None => "something unexpected happened! please report this",
	Some(message) => message,
})]
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
		Self { status, message: None, location: Location::caller(), source: None }
	}

	/// Set the message of the error.
	pub fn with_message(mut self, message: impl Display) -> Self {
		self.message = Some(message.to_string());
		self
	}

	/// Set the source of the error.
	pub fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
		self.source = Some(Box::new(source));
		self
	}

	/// An error caused by a TCP socket.
	#[track_caller]
	pub fn tcp(error: io::Error) -> Self {
		Self::bug("failed to listen on tcp socket").with_source(error)
	}

	/// An error caused by axum / hyper's server implementation.
	#[track_caller]
	pub fn http_server(error: io::Error) -> Self {
		Self::bug("failed to start http server").with_source(error)
	}

	/// An unexpected error.
	#[track_caller]
	pub fn bug(message: impl Display) -> Self {
		error!(target: "audit_log", %message, "unexpected runtime error");
		Self::new(StatusCode::INTERNAL_SERVER_ERROR).with_message(message)
	}

	/// `204 No Content` status code.
	#[track_caller]
	pub fn no_content() -> Self {
		Self::new(StatusCode::NO_CONTENT)
	}

	/// `401 Unauthorized` status code.
	#[track_caller]
	pub fn unauthorized() -> Self {
		Self::new(StatusCode::UNAUTHORIZED)
	}

	/// Some user input (e.g. an ID) is unknown / invalid.
	#[track_caller]
	pub fn unknown(what: impl Display) -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message(format_args!("unknown {what}"))
	}

	/// Some user input in a POST / PUT request already exists in the database.
	#[track_caller]
	pub fn already_exists(what: impl Display) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format_args!("{what} already exists"))
	}

	/// When PATCHing maps, the user shouldn't be allowed to remove all mappers from a map /
	/// course.
	#[track_caller]
	pub fn must_have_mappers() -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message("map/course cannot have 0 mappers")
	}

	/// A CS2 server tried to request an access key (JWT) but their supplied refresh key was
	/// invalid.
	#[track_caller]
	pub fn invalid_refresh_key() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("refresh key is invalid")
	}

	/// A CS2 server made an authenticated request but their access key was expired.
	#[track_caller]
	pub fn expired_access_key() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("access key is expired")
	}

	/// A user tried to make an authenticated request but was missing their session token.
	#[track_caller]
	pub fn missing_session_token() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("missing session token")
	}

	/// A user tried to make an authenticated request but their session token was invalid or
	/// expired.
	#[track_caller]
	pub fn invalid_session_token() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("invalid session token")
	}

	/// A user tried to make an authenticated request but didn't have the required roles.
	#[track_caller]
	pub fn missing_roles(roles: RoleFlags) -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message(format_args!(
			"you are missing the required roles to perform this action ({roles})"
		))
	}

	/// A user tried to make an authenticated request but weren't the owner of the server they
	/// tried to PATCH.
	#[track_caller]
	pub fn must_be_server_owner() -> Self {
		Self::new(StatusCode::UNAUTHORIZED)
			.with_message("you must be the server's owner or an admin to perform this action")
	}

	/// An opaque API key was not a valid UUID.
	#[track_caller]
	pub fn key_must_be_uuid(error: uuid::Error) -> Self {
		Self::new(StatusCode::BAD_REQUEST)
			.with_message("key must be a valid UUID")
			.with_source(error)
	}

	/// An opaque API key was invalid.
	#[track_caller]
	pub fn key_invalid() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("key is invalid")
	}

	/// An opaque API key was expired.
	#[track_caller]
	pub fn key_expired() -> Self {
		Self::new(StatusCode::UNAUTHORIZED).with_message("key has expired")
	}
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		debug!(location = %self.location, "runtime error occurred");

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
			error => {
				error!(target: "audit_log", %error, "database error");
				Self::bug("database error").with_source(error)
			}
		}
	}
}

impl From<jsonwebtoken::errors::Error> for Error {
	#[track_caller]
	fn from(error: jsonwebtoken::errors::Error) -> Self {
		error!(target: "audit_log", %error, "failed to (de)serialize jwt");
		Self::new(StatusCode::INTERNAL_SERVER_ERROR).with_source(error)
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
