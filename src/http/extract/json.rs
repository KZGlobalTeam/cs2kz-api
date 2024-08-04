//! This module contains the [`Json`] extractor, a wrapper around [`axum::Json`]
//! with a custom error response.

use axum::async_trait;
use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;

#[allow(clippy::missing_docs_in_private_items)]
mod base
{
	pub use axum::extract::rejection::JsonRejection;
	pub use axum::Json;
}

/// An extractor for JSON request bodies.
///
/// This wraps [`axum::Json`] exactly, but produces different error responses.
#[derive(Debug)]
pub struct Json<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for Json<T>
where
	S: Send + Sync,
	T: DeserializeOwned,
{
	type Rejection = JsonRejection;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection>
	{
		if !has_json_content_type(req.headers()) {
			return Err(base::JsonRejection::MissingJsonContentType(Default::default()).into());
		}

		let bytes = Bytes::from_request(req, state)
			.await
			.map_err(base::JsonRejection::BytesRejection)?;

		serde_json::from_slice(&bytes).map(Self).map_err(Into::into)
	}
}

impl<T> IntoResponse for Json<T>
where
	base::Json<T>: IntoResponse,
{
	fn into_response(self) -> Response
	{
		base::Json(self.0).into_response()
	}
}

/// Rejection for the [`Json`] extractor.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum JsonRejection
{
	#[error(transparent)]
	Base(#[from] base::JsonRejection),

	#[error(transparent)]
	Deserialize(#[from] serde_json::Error),
}

impl IntoResponse for JsonRejection
{
	fn into_response(self) -> Response
	{
		ProblemDetails::from(self).into_response()
	}
}

impl IntoProblemDetails for JsonRejection
{
	fn problem_type(&self) -> ProblemType
	{
		use serde_json::error::Category as ECategory;

		match self {
			Self::Base(base::JsonRejection::MissingJsonContentType(_)) => {
				ProblemType::MissingHeader
			}
			Self::Base(
				base::JsonRejection::JsonDataError(_)
				| base::JsonRejection::BytesRejection(_)
				| base::JsonRejection::JsonSyntaxError(_),
			) => ProblemType::InvalidRequestBody,
			Self::Deserialize(source) => match source.classify() {
				ECategory::Io => unreachable!(),
				ECategory::Syntax | ECategory::Data | ECategory::Eof => {
					ProblemType::InvalidRequestBody
				}
			},

			_ => ProblemType::InvalidRequestBody,
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		if let Self::Deserialize(source) = self {
			ext.add("line", &source.line());
			ext.add("column", &source.column());
		}
	}
}

/// Checks if the given `headers` contain a JSON-like Content-Type.
fn has_json_content_type(headers: &http::HeaderMap) -> bool
{
	let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
		return false;
	};

	let Ok(content_type) = content_type.to_str() else {
		return false;
	};

	let Ok(mime) = content_type.parse::<mime::Mime>() else {
		return false;
	};

	mime.type_() == "application"
		&& (mime.subtype() == "json" || mime.suffix().map_or(false, |name| name == "json"))
}
