// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::middleware::server_auth::{API_KEY_HEADER, API_TOKEN_HEADER},
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

	#[error("Database error.")]
	Database,

	#[error("Missing `{}` header.", API_KEY_HEADER)]
	MissingApiKey,

	#[error("Invalid `{}` header.", API_KEY_HEADER)]
	InvalidApiKey,

	#[error("Missing `{}` header.", API_TOKEN_HEADER)]
	MissingApiToken,

	#[error("Invalid `{}` header.", API_TOKEN_HEADER)]
	InvalidApiToken,

	#[error("Your server has an outdated KZ plugin. Please update.")]
	OutdatedPluginVersion,

	#[error("Failed to parse request body.")]
	InvalidRequestBody,

	#[error("You are not authorized to perform this action.")]
	Unauthorized,

	#[error("Submitted map is invalid.")]
	InvalidMap,

	#[error("Submitted record does not have a matching filter.")]
	InvalidFilter,
}

impl IntoResponse for Error {
	fn into_response(self) -> AxumResponse {
		let message = self.to_string();
		let code = match self {
			Self::InternalServerError | Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
			Self::MissingApiKey
			| Self::InvalidApiKey
			| Self::MissingApiToken
			| Self::InvalidApiToken
			| Self::Unauthorized
			| Self::OutdatedPluginVersion => StatusCode::UNAUTHORIZED,

			Self::InvalidRequestBody | Self::InvalidMap | Self::InvalidFilter => {
				StatusCode::BAD_REQUEST
			}
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
