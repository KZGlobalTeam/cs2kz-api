use std::fmt::Display;
use std::panic::Location;
use std::result::Result as StdResult;

use axum::extract::rejection::QueryRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::typed_header::TypedHeaderRejection;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use thiserror::Error as ThisError;
use tracing::warn;
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

use crate::{audit, state, Config};

pub type Result<T> = StdResult<T, Error>;

/// Global error type for request handlers and middleware.
///
/// Any errors that can occurr during runtime will be transformed into this type before turning
/// into an HTTP response.
#[derive(Debug, ThisError)]
#[error("Failed with code {}{}", code.as_u16(), match message {
	None => String::new(),
	Some(message) => format!(": {message}"),
})]
pub struct Error {
	/// HTTP Status Code to return in the response.
	code: StatusCode,

	/// Concise description of the error.
	///
	/// This should be human-readable and will be included in the response body.
	message: Option<String>,

	/// Additional details about the error.
	detail: JsonValue,

	/// The source code location of where the error occurred (used for debugging).
	location: &'static Location<'static>,
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		warn! {
			message = ?self.message,
			detail = ?self.detail,
			location = %self.location,
			"encountered runtime error",
		};

		let mut body = json!({
			"message": self.message
		});

		if !self.detail.is_null() {
			body["detail"] = self.detail;
		}

		(self.code, Json(body)).into_response()
	}
}

impl Error {
	/// Set the `message` field for this error.
	pub fn with_message(mut self, message: impl Display) -> Self {
		self.message = Some(message.to_string());
		self
	}

	/// Set the `detail` field for this error.
	///
	/// If the inner `detail` already exists, and both it and the supplied `detail` are
	/// objects, it will be extended with the provided `detail` value. Otherwise the inner
	/// `detail` will be replaced.
	pub fn with_detail(mut self, detail: impl Serialize) -> Self {
		let value = serde_json::to_value(detail).expect("invalid json value");

		match (&mut self.detail, value) {
			(JsonValue::Object(detail), JsonValue::Object(obj)) => {
				detail.extend(obj);
			}
			(detail, value) => {
				*detail = value;
			}
		}

		self
	}

	/// Set the status code to `404 Unauthorized`.
	pub fn unauthorized(mut self) -> Self {
		self.code = StatusCode::UNAUTHORIZED;
		self
	}

	/// Generate an error indicating a bug in the application.
	#[track_caller]
	pub fn bug() -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR)
	}

	/// Indicate that something about the request is missing.
	#[track_caller]
	pub fn missing(what: impl Display) -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message(format_args!("missing {what}"))
	}

	/// Indicate that something about the request is unknown.
	#[track_caller]
	pub fn unknown(what: impl Display) -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message(format_args!("unknown {what}"))
	}

	/// Indicate that a supplied ID is unknown.
	///
	/// This is a convenience wrapper around [`Error::unknown()`].
	#[track_caller]
	pub fn unknown_id(what: impl Display, id: impl Serialize) -> Self {
		Self::unknown(format_args!("{what} ID")).with_detail(json!({ "id": id }))
	}

	/// Indicate that something about the request is invalid.
	#[track_caller]
	pub fn invalid(what: impl Display) -> Self {
		Self::new(StatusCode::BAD_REQUEST).with_message(format_args!("invalid {what}"))
	}

	/// Indicate that there is no data to return in the response.
	#[track_caller]
	pub fn no_data() -> Self {
		Self::new(StatusCode::NO_CONTENT)
	}

	/// Indicate that the download process of a Steam Workshop map has failed.
	#[track_caller]
	pub fn download_workshop_map() -> Self {
		Self::bug().with_message("failed to download workshop map")
	}

	/// Generate a new error with the given status `code`.
	#[track_caller]
	fn new(code: StatusCode) -> Self {
		Self {
			code,
			message: code.canonical_reason().map(ToOwned::to_owned),
			detail: JsonValue::Null,
			location: Location::caller(),
		}
	}
}

impl From<state::Error> for Error {
	#[track_caller]
	fn from(error: state::Error) -> Self {
		use state::Error as E;

		match error {
			E::MySQL(err) => Self::from(err),
			E::JsonEncode(err) => {
				audit!(error, "failed to serialize JSON", %err);

				if Config::environment().is_dev() {
					Self::bug()
						.with_message("failed to serialize JSON")
						.with_detail(err.to_string())
				} else {
					Self::bug()
				}
			}
			E::JsonDecode(err) => Self::new(StatusCode::BAD_REQUEST)
				.with_message("failed to decode JSON")
				.with_detail(err.to_string()),
			E::Jwt(err) => {
				unreachable!("not reachable after startup: {err}");
			}
			E::JwtEncode(err) => {
				audit!(error, "failed to encode JWT", %err);

				if Config::environment().is_dev() {
					Self::bug()
						.with_message("failed to encode JWT")
						.with_detail(err.to_string())
				} else {
					Self::bug()
				}
			}
			E::JwtDecode(err) => Self::new(StatusCode::BAD_REQUEST)
				.with_message("failed to decode JWT")
				.with_detail(err.to_string()),
		}
	}
}

impl From<TypedHeaderRejection> for Error {
	#[track_caller]
	fn from(rejection: TypedHeaderRejection) -> Self {
		use axum_extra::typed_header::TypedHeaderRejectionReason as Reason;

		let error = Self::new(StatusCode::BAD_REQUEST)
			.with_message(format_args!("failed to decode header `{}`", rejection.name()));

		match rejection.reason() {
			Reason::Missing => error.with_detail("header is missing"),
			Reason::Error(err) => error.with_detail(err.to_string()),
			_ => error,
		}
	}
}

impl From<QueryRejection> for Error {
	#[track_caller]
	fn from(rejection: QueryRejection) -> Self {
		Self::new(rejection.status())
			.with_message("failed to decode query parameters")
			.with_detail(rejection.body_text())
	}
}

impl From<reqwest::Error> for Error {
	#[track_caller]
	fn from(error: reqwest::Error) -> Self {
		Self::new(StatusCode::BAD_GATEWAY)
			.with_message("request to external service failed")
			.with_detail(json!({
				"code": error.status().map(|code| code.as_u16()),
				"message": error.to_string()
			}))
	}
}

impl From<sqlx::Error> for Error {
	#[track_caller]
	fn from(error: sqlx::Error) -> Self {
		audit!(error, "database error", %error);

		Self::bug().with_message("Encountered database error. This is a bug, please report it.")
	}
}

impl<'s> ToSchema<'s> for Error {
	fn schema() -> (&'s str, RefOr<Schema>) {
		(
			"Error",
			ObjectBuilder::new()
				.property("message", ObjectBuilder::new().schema_type(SchemaType::String))
				.required("message")
				.property("detail", ObjectBuilder::new().schema_type(SchemaType::Value))
				.into(),
		)
	}
}
