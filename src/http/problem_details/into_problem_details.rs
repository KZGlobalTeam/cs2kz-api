//! This module contains the [`IntoProblemDetails`] trait.
//!
//! It defines the contract for how an error type can be turned into an HTTP
//! error response.

use std::convert;

use axum_extra::typed_header::TypedHeaderRejection;

use super::{ExtensionMembers, ProblemType};

/// A trait for creating [`ProblemDetails`] from error types.
///
/// [`ProblemDetails`]: super::ProblemDetails
pub trait IntoProblemDetails: std::error::Error
{
	/// Returns the problem type for this error.
	fn problem_type(&self) -> ProblemType;

	/// Adds [extension members] to the HTTP response.
	///
	/// [extension members]: https://www.rfc-editor.org/rfc/rfc9457.html#name-extension-members
	fn add_extension_members(&self, ext: &mut ExtensionMembers)
	{
		_ = ext;
	}
}

impl IntoProblemDetails for convert::Infallible
{
	fn problem_type(&self) -> ProblemType
	{
		match *self {}
	}
}

impl IntoProblemDetails for sqlx::Error
{
	fn problem_type(&self) -> ProblemType
	{
		ProblemType::Internal
	}
}

impl IntoProblemDetails for reqwest::Error
{
	fn problem_type(&self) -> ProblemType
	{
		if self.is_connect() {
			return ProblemType::Internal;
		}

		if self.is_body() || self.is_decode() {
			return ProblemType::DecodeExternal;
		}

		if self.is_redirect() || self.is_timeout() {
			return ProblemType::ExternalService;
		}

		match self.status() {
			None => ProblemType::Internal,
			Some(status) if status.is_server_error() => ProblemType::ExternalService,
			Some(_) => ProblemType::Internal,
		}
	}
}

impl IntoProblemDetails for TypedHeaderRejection
{
	fn problem_type(&self) -> ProblemType
	{
		if self.is_missing() {
			ProblemType::MissingHeader
		} else {
			ProblemType::InvalidHeader
		}
	}
}
