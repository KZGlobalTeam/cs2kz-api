use {
	axum::{
		http::StatusCode,
		response::{IntoResponse, Response as AxumResponse},
		Json,
	},
	thiserror::Error as ThisError,
	tracing::error,
	utoipa::ToSchema,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, ThisError, ToSchema)]
pub enum Error {
	#[error("Something went wrong.")]
	InternalServerError,

	#[error("No data found matching query.")]
	NoContent,
}

impl IntoResponse for Error {
	fn into_response(self) -> AxumResponse {
		let message = self.to_string();
		let code = match self {
			Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
			Self::NoContent => StatusCode::NO_CONTENT,
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
