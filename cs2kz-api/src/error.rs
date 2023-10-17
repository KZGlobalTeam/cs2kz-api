// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

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
	#[error("Database error.")]
	Database,

	#[error("Missing `api-token` header.")]
	MissingToken,

	#[error("Invalid `api-token` header.")]
	InvalidToken,

	#[error("Failed to parse request body.")]
	InvalidRequestBody,

	#[error("You are not authorized to perform this action.")]
	Unauthorized,
}

impl IntoResponse for Error {
	fn into_response(self) -> AxumResponse {
		let message = self.to_string();
		let code = match self {
			Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
			Self::MissingToken | Self::Unauthorized => StatusCode::UNAUTHORIZED,
			Self::InvalidToken | Self::InvalidRequestBody => StatusCode::BAD_REQUEST,
		};

		(code, Json(message)).into_response()
	}
}

impl From<sqlx::Error> for Error {
	fn from(error: sqlx::Error) -> Self {
		error!(?error, "Database error");

		Self::Database
	}
}

impl From<hyper::Error> for Error {
	fn from(_error: hyper::Error) -> Self {
		Self::InvalidRequestBody
	}
}
