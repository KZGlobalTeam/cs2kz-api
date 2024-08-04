//! This module contains a configured [`CatchPanicLayer`], which will catch
//! panics and produce a proper HTTP response from them.
//!
//! Normally, if an HTTP handler panics, the connection will simply be closed.
//! This middleware will prevent that from happening.

use std::any::Any;

use axum::response::IntoResponse;
use thiserror::Error;
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

use crate::http::problem_details::{IntoProblemDetails, ProblemType};
use crate::http::ProblemDetails;

/// Creates a middleware layer for catching panics and turning them into
/// responses.
pub fn layer() -> CatchPanicLayer<PanicHandler>
{
	CatchPanicLayer::custom(PanicHandler)
}

/// A custom panic handler for [`CatchPanicLayer`].
#[derive(Clone)]
pub struct PanicHandler;

/// An error type describing that an HTTP handler panicked.
#[derive(Debug, Clone, Error)]
#[error("something unexpected happened; please report this incident")]
struct HandlerPanicked;

impl IntoProblemDetails for HandlerPanicked
{
	fn problem_type(&self) -> ProblemType
	{
		ProblemType::Internal
	}
}

impl ResponseForPanic for PanicHandler
{
	type ResponseBody = axum::body::Body;

	#[tracing::instrument(target = "cs2kz_api::http::middleware", name = "panic_handler", skip_all)]
	fn response_for_panic(
		&mut self,
		error: Box<dyn Any + Send + 'static>,
	) -> http::Response<Self::ResponseBody>
	{
		let error = error
			.downcast_ref::<&str>()
			.copied()
			.or_else(|| error.downcast_ref::<String>().map(|s| s.as_str()));

		tracing::error!(?error, "handler panicked");

		ProblemDetails::from(HandlerPanicked).into_response()
	}
}
