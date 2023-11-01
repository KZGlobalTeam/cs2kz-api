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
}

impl IntoResponse for Error {
	fn into_response(self) -> AxumResponse {
		let message = self.to_string();
		let code = match self {
			Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
		};

		(code, Json(message)).into_response()
	}
}
