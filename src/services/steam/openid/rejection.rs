//! Rejection types for [`OpenIDPayload`].
//!
//! [`OpenIDPayload`]: super::OpenIDPayload

use axum::response::{IntoResponse, Response};
use axum_extra::extract::QueryRejection;
use thiserror::Error;

use crate::http::problem_details::{IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;

/// Rejections for the [`OpenIDPayload`] extractor.
///
/// [`OpenIDPayload`]: super::OpenIDPayload
#[derive(Debug, Error)]
pub enum OpenIDRejection
{
	/// We failed to extract OpenID query parameters from an incoming request.
	#[error(transparent)]
	Query(#[from] QueryRejection),

	/// We failed to make an HTTP request to Steam.
	#[error("failed to make http request")]
	Http(#[from] reqwest::Error),

	/// An OpenID payload we sent to Steam for verification came back as
	/// invalid.
	#[error("failed to verify openid payload with Steam")]
	VerifyOpenIDPayload,
}

impl IntoProblemDetails for OpenIDRejection
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::Query(_) => ProblemType::InvalidQueryString,
			Self::Http(source) => source.problem_type(),
			Self::VerifyOpenIDPayload => ProblemType::InvalidOpenIDPayload,
		}
	}
}

impl IntoResponse for OpenIDRejection
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}
