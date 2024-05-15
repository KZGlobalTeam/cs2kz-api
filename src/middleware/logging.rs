//! Middleware for logging HTTP requests & responses.

use std::time::Duration;

use axum::extract::Request;
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use tracing::{debug, error, warn};
use uuid::Uuid;

/// A tower layer that will log HTTP requests & responses.
macro_rules! layer {
	() => {
		tower_http::trace::TraceLayer::new_for_http()
			.make_span_with($crate::middleware::logging::make_span_with)
			.on_response($crate::middleware::logging::on_response)
			.on_failure($crate::middleware::logging::on_failure)
	};
}

pub(crate) use layer;

#[doc(hidden)]
pub(crate) fn make_span_with(request: &Request) -> tracing::Span {
	tracing::trace_span! {
		target: "cs2kz_api::logs",
		"request",
		request.id = %Uuid::new_v4(),
		request.method = %request.method(),
		request.path = format_args!("`{}`", request.uri()),
		request.version = ?request.version(),
		request.headers = ?request.headers(),
		response.status = tracing::field::Empty,
		response.headers = tracing::field::Empty,
		latency = tracing::field::Empty,
	}
}

#[doc(hidden)]
pub(crate) fn on_response(response: &Response, latency: Duration, span: &tracing::Span) {
	span.record("response.status", format_args!("{}", response.status()))
		.record("response.headers", format_args!("{:?}", response.headers()))
		.record("latency", format_args!("{:?}", latency));
}

#[doc(hidden)]
pub(crate) fn on_failure(
	failure: ServerErrorsFailureClass,
	_latency: Duration,
	_span: &tracing::Span,
) {
	match failure {
		ServerErrorsFailureClass::Error(error) => {
			warn!(target: "audit_log", %error, "request handler failed");
		}
		ServerErrorsFailureClass::StatusCode(code) if code.is_server_error() => {
			error!(target: "audit_log", %code, "request handler failed");
		}
		ServerErrorsFailureClass::StatusCode(code) if code.is_client_error() => {
			debug!(target: "audit_log", %code, "request handler failed");
		}
		ServerErrorsFailureClass::StatusCode(_) => {}
	}
}
