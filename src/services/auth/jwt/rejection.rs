//! Rejection types for [`Jwt`]
//!
//! [`Jwt`]: super::Jwt

use axum::response::{IntoResponse, Response};
use axum_extra::typed_header::TypedHeaderRejection;
use thiserror::Error;

use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;
use crate::services::auth;

/// Rejection for extracing a [`Jwt`] from a request.
///
/// [`Jwt`]: super::Jwt
#[derive(Debug, Error)]
pub enum JwtRejection
{
	/// The `Authorization` header was missing / malformed.
	#[error("failed to decode `Authorization` header: {0}")]
	Header(#[from] TypedHeaderRejection),

	/// The JWT exists, but has already expired.
	#[error("token has expired")]
	JwtExpired,

	/// The auth service failed for some reason.
	#[error(transparent)]
	Auth(#[from] auth::Error),
}

impl IntoProblemDetails for JwtRejection
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::Header(source) => source.problem_type(),
			Self::JwtExpired => ProblemType::Unauthorized,
			Self::Auth(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		if let Self::Auth(source) = self {
			source.add_extension_members(ext);
		}
	}
}

impl IntoResponse for JwtRejection
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}
