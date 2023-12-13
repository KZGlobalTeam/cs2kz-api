//! This module holds structs that will be returned from handlers.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

/// Wrapper that, when turned into a [`Response`], forces a status code of `201 CREATED`.
///
/// [`Response`]: axum::response::Response
#[derive(Debug, Clone, Serialize)]
pub struct Created<T: IntoResponse = ()>(pub T);

impl<T: IntoResponse> IntoResponse for Created<T> {
	fn into_response(self) -> axum::response::Response {
		(StatusCode::CREATED, self.0).into_response()
	}
}
