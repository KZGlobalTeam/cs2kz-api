//! HTTP Problem Details [RFC 9457].
//!
//! The [`ProblemDetails`] can be used for HTTP responses, as it implements
//! [`IntoResponse`]. It can be created from any error type that implements
//! [`IntoProblemDetails`].
//!
//! [RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html

use std::panic::Location;

use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tap::Tap;

pub(crate) mod problem_type;
pub use problem_type::ProblemType;

mod extension_members;
pub use extension_members::ExtensionMembers;

mod into_problem_details;
pub use into_problem_details::IntoProblemDetails;

/// HTTP Problem Details, as described in [RFC 9457].
///
/// [RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html
#[derive(Debug, Serialize)]
pub struct ProblemDetails
{
	/// The problem type.
	#[serde(rename = "type")]
	problem_type: ProblemType,

	/// The HTTP status code the response should have.
	#[serde(skip_serializing)]
	status: http::StatusCode,

	/// Short, human-readable, description of the problem type.
	title: &'static str,

	/// Short, human-readable, error message describing this particular problem.
	detail: String,

	/// Any extra details that will be included in the response body.
	#[serde(flatten)]
	extra: ExtensionMembers,
}

impl<E> From<E> for ProblemDetails
where
	E: IntoProblemDetails,
{
	#[track_caller]
	fn from(error: E) -> Self
	{
		tracing::debug!(loc = %Location::caller(), ?error, "creating error response");

		let problem_type = error.problem_type();
		let status = problem_type.status();
		let title = problem_type.title();
		let detail = error.to_string();
		let extra = ExtensionMembers::new().tap_mut(|ext| {
			error.add_extension_members(ext);
		});

		Self { problem_type, status, title, detail, extra }
	}
}

impl IntoResponse for ProblemDetails
{
	fn into_response(self) -> Response
	{
		let status = self.status;
		let content_type = "application/problem+json";
		let headers = [(http::header::CONTENT_TYPE, content_type)];
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}

/// Trait implementations for [`utoipa`].
mod utoipa_impls
{
	use std::collections::BTreeMap;

	use itertools::Itertools;
	use utoipa::openapi::response::{Response, ResponseBuilder, ResponsesBuilder};
	use utoipa::openapi::RefOr;
	use utoipa::IntoResponses;

	use super::{ProblemDetails, ProblemType};

	impl IntoResponses for ProblemDetails
	{
		fn responses() -> BTreeMap<String, RefOr<Response>>
		{
			let problems = ProblemType::all();
			let statuses = problems
				.iter()
				.map(|problem| problem.status())
				.collect_vec();

			let responses = statuses
				.iter()
				.map(|status| (status.as_str(), status.canonical_reason().unwrap_or_default()))
				.map(|(code, reason)| (code, ResponseBuilder::new().description(reason).build()));

			ResponsesBuilder::new()
				.responses_from_iter(responses)
				.build()
				.into()
		}
	}
}
