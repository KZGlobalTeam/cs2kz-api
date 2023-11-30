use {
	axum::{
		http::StatusCode,
		response::{IntoResponse, Response as AxumResponse},
		Json,
	},
	serde::Serialize,
	serde_json::json,
	thiserror::Error as ThisError,
	tracing::error,
	utoipa::ToSchema,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, ThisError, Serialize, ToSchema)]
#[serde(tag = "message")]
pub enum Error {
	#[error("Something went wrong.")]
	InternalServerError,

	#[error("No data found matching query.")]
	NoContent,

	#[error("Missing course for stage {stage}.")]
	MissingCourse { stage: u8 },

	#[error("Filter for stage {stage} is invalid as stage {stage} does not exist.")]
	InvalidFilter { stage: u8 },

	#[error("Cannot create duplicate course for stage {stage}.")]
	DuplicateCourse { stage: u8 },

	#[error("Cannot create duplicate filter for stage {stage}.")]
	DuplicateFilter { stage: u8 },

	#[error("Filter for this record does not exist.")]
	MissingFilter,

	#[error("You don't have access to this resource.")]
	Unauthorized,

	#[error("Invalid request body.")]
	InvalidRequestBody,
}

impl IntoResponse for Error {
	fn into_response(self) -> AxumResponse {
		let message = json! {
			{
				"message": self.to_string()
			}
		};

		let code = match self {
			Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
			Self::NoContent => StatusCode::NO_CONTENT,
			Self::MissingCourse { .. }
			| Self::InvalidFilter { .. }
			| Self::DuplicateCourse { .. }
			| Self::DuplicateFilter { .. }
			| Self::MissingFilter
			| Self::InvalidRequestBody => StatusCode::BAD_REQUEST,
			Self::Unauthorized => StatusCode::UNAUTHORIZED,
		};

		(code, Json(message)).into_response()
	}
}

impl From<sqlx::Error> for Error {
	fn from(error: sqlx::Error) -> Self {
		error!(?error, "database error");
		Self::InternalServerError
	}
}

impl From<jsonwebtoken::errors::Error> for Error {
	fn from(error: jsonwebtoken::errors::Error) -> Self {
		error!(error = ?error.kind(), "failed to decode jwt");

		Self::Unauthorized
	}
}
