//! Middleware for logging HTTP requests & responses.

use std::time::Duration;

use axum::extract::Request;
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use tracing::{error, Level, Span};
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
pub(crate) fn make_span_with(request: &Request) -> Span {
	tracing::span! {
		Level::TRACE,
		"request",
		id = %Uuid::new_v4(),
		method = %request.method(),
		path = %request.uri(),
		version = ?request.version(),
		request.headers = ?request.headers(),
		status = tracing::field::Empty,
		response.headers = tracing::field::Empty,
		latency = tracing::field::Empty,
	}
}

#[doc(hidden)]
pub(crate) fn on_response(response: &Response, latency: Duration, span: &Span) {
	span.record("status", format_args!("{}", response.status()))
		.record("response.headers", format_args!("{:?}", response.headers()))
		.record("latency", format_args!("{:?}", latency));
}

#[doc(hidden)]
pub(crate) fn on_failure(failure: ServerErrorsFailureClass, _latency: Duration, _span: &Span) {
	match failure {
		ServerErrorsFailureClass::Error(message) => {
			error!(target: "audit_log", %message, "encountered runtime error");
		}
		ServerErrorsFailureClass::StatusCode(code) if code.is_server_error() => {
			error!(target: "audit_log", code = format_args!("{code}"), "encountered runtime error");
		}
		ServerErrorsFailureClass::StatusCode(_) => {}
	}
}
