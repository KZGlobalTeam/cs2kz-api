use std::result::Result as StdResult;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as AxumResponse};
use axum::Json;
use jsonwebtoken as jwt;
use serde::{Serialize, Serializer};
use serde_json::json;
use thiserror::Error as ThisError;
use tracing::error;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, ThisError)]
pub enum Error {
	#[error("There is no data available for this query.")]
	NoContent,

	#[error("Invalid request body.")]
	InvalidRequestBody,

	#[error("You do not have access to this resource.")]
	Unauthorized,

	#[error("Missing course for stage {stage}.")]
	MissingCourse { stage: u8 },

	#[error("Cannot create duplicate course for stage {stage}.")]
	DuplicateCourse { stage: u8 },

	#[error("Filter for this record does not exist.")]
	MissingFilter,

	#[error("Something went wrong. This is a bug.")]
	InternalServerError,
}

impl IntoResponse for Error {
	fn into_response(self) -> AxumResponse {
		let code = match self {
			Self::NoContent => StatusCode::NO_CONTENT,
			Self::InvalidRequestBody
			| Self::MissingCourse { .. }
			| Self::DuplicateCourse { .. }
			| Self::MissingFilter => StatusCode::BAD_REQUEST,
			Self::Unauthorized => StatusCode::UNAUTHORIZED,
			Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
		};

		(code, Json(self)).into_response()
	}
}

impl Serialize for Error {
	fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// TODO(AlphaKeks): include a custom error code here?
		json!({ "message": self.to_string() }).serialize(serializer)
	}
}

impl From<sqlx::Error> for Error {
	fn from(error: sqlx::Error) -> Self {
		error!(?error, "database error");
		Self::InternalServerError
	}
}

impl From<jwt::errors::Error> for Error {
	fn from(error: jwt::errors::Error) -> Self {
		error!(error = ?error.kind(), "failed to decode jwt");

		Self::Unauthorized
	}
}
