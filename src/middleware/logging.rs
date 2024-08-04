//! This module contains configuration for the [`Trace`] middleware from
//! [`tower_http`].
//!
//! As this middleware is configurable, we implement custom hooks for creating
//! the tracing span, logging responses, etc.
//!
//! The resulting type contains unique function types, and as such cannot be
//! spelled out in code. This is why we export a macro instead, which will just
//! expand to the final expression. This also means that the `pub(crate)`
//! functions in this module aren't really meant to be `pub`. They need to be,
//! so that the macro can call them, but they are marked as `#[doc(hidden)]` so
//! nobody is tempted to use them for anything else.
//!
//! [`Trace`]: tower_http::trace::Trace

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, Request};
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use uuid::Uuid;

/// Creates a middleware that will log incoming HTTP requests.
///
/// It will attach a unique ID to every tracing span and log metadata such as
/// the request head, and response status.
macro_rules! layer {
	() => {
		tower_http::trace::TraceLayer::new_for_http()
			.make_span_with($crate::middleware::logging::make_span)
			.on_response($crate::middleware::logging::on_response)
			.on_failure($crate::middleware::logging::on_failure)
	};
}

pub(crate) use layer;

#[doc(hidden)]
pub(crate) fn make_span(request: &Request) -> tracing::Span
{
	let ip = match request.extensions().get::<ConnectInfo<SocketAddr>>() {
		None => String::from("N/A"),
		Some(ConnectInfo(addr)) => addr.to_string(),
	};

	tracing::info_span! {
		target: "cs2kz_api::http",
		"request",
		request.id = %Uuid::now_v7(),
		request.ip = %ip,
		request.method = %request.method(),
		request.uri = %request.uri(),
		request.version = ?request.version(),
		request.headers = ?request.headers(),
		response.status = tracing::field::Empty,
		response.headers = tracing::field::Empty,
		latency = tracing::field::Empty,
	}
}

#[doc(hidden)]
pub(crate) fn on_response(response: &Response, latency: Duration, span: &tracing::Span)
{
	span.record("response.status", format_args!("{}", response.status()))
		.record("response.headers", format_args!("{:?}", response.headers()))
		.record("latency", format_args!("{:?}", latency));
}

#[doc(hidden)]
pub(crate) fn on_failure(
	failure: ServerErrorsFailureClass,
	_latency: Duration,
	_span: &tracing::Span,
)
{
	match failure {
		ServerErrorsFailureClass::Error(error) => {
			tracing::error!(target: "cs2kz_api::audit_log", %error, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) if status.is_server_error() => {
			tracing::error!(target: "cs2kz_api::audit_log", %status, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) if status.is_client_error() => {
			tracing::debug!(target: "cs2kz_api::audit_log", %status, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) => {
			tracing::warn!(target: "cs2kz_api::audit_log", %status, "error occurred during request");
		}
	}
}
