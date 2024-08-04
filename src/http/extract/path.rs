//! This module contains the [`Path`] extractor, a wrapper around
//! [`axum::extract::Path`] with a custom error response.

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;

#[allow(clippy::missing_docs_in_private_items)]
mod base
{
	pub use axum::extract::path::ErrorKind;
	pub use axum::extract::rejection::PathRejection;
	pub use axum::extract::Path;
}

/// An extractor for URI segment captures.
///
/// This wraps [`axum::extract::Path`] exactly, but produces different error
/// responses.
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Path), rejection(PathRejection))]
pub struct Path<T>(pub T);

/// Rejection for the [`Path`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct PathRejection(#[from] pub base::PathRejection);

impl IntoResponse for PathRejection
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}

impl IntoProblemDetails for PathRejection
{
	fn problem_type(&self) -> ProblemType
	{
		match self.0 {
			base::PathRejection::MissingPathParams(_) => ProblemType::MissingPathParameters,
			base::PathRejection::FailedToDeserializePathParams(_) => {
				ProblemType::InvalidPathParameters
			}
			_ => ProblemType::InvalidPathParameters,
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		let base::PathRejection::FailedToDeserializePathParams(source) = &self.0 else {
			return;
		};

		match source.kind() {
			base::ErrorKind::WrongNumberOfParameters { got, expected } => {
				ext.add("got", &got);
				ext.add("expected", &expected);
			}
			base::ErrorKind::ParseErrorAtKey { key, value, expected_type } => {
				ext.add("parameter", &key);
				ext.add("value", &value);
				ext.add("expected_type", expected_type);
			}
			base::ErrorKind::ParseErrorAtIndex { index, value, expected_type } => {
				ext.add("idx", &index);
				ext.add("value", &value);
				ext.add("expected_type", expected_type);
			}
			base::ErrorKind::ParseError { value, expected_type } => {
				ext.add("value", &value);
				ext.add("expected_type", expected_type);
			}
			base::ErrorKind::InvalidUtf8InPathParam { key } => {
				ext.add("parameter", &key);
			}
			base::ErrorKind::UnsupportedType { name } => {
				ext.add("unsupported_type", name);
			}

			_ => {}
		}
	}
}
