//! Custom [responses].
//!
//! [responses]: axum::response

use axum::response::{IntoResponse, Response};

use crate::extract::Json;

mod error;
pub use error::ErrorResponse;

#[derive(Debug)]
pub struct Created<T>(pub T)
where
    Json<T>: IntoResponse;

impl<T> IntoResponse for Created<T>
where
    Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        (http::StatusCode::CREATED, Json(self.0)).into_response()
    }
}
