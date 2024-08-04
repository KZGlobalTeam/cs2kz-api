//! This module contains the [`Query`] extractor, a wrapper around
//! [`axum_extra::extract::Query`] with a custom error response.

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::http::problem_details::{IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;

#[allow(clippy::missing_docs_in_private_items)]
mod base
{
	pub use axum_extra::extract::{Query, QueryRejection};
}

/// An extractor for URI query parameters.
///
/// This wraps [`axum_extra::extract::Query`] exactly, but produces different
/// error responses.
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Query), rejection(QueryRejection))]
pub struct Query<T>(pub T);

/// Rejection for the [`Query`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct QueryRejection(#[from] pub base::QueryRejection);

impl IntoResponse for QueryRejection
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}

impl IntoProblemDetails for QueryRejection
{
	fn problem_type(&self) -> ProblemType
	{
		ProblemType::InvalidQueryString
	}
}
