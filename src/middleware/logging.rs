use std::time::Duration;

use axum::body::Bytes;
use axum::extract::Request;
use axum::response::Response;
use tracing::Level;

/// Middleware for logging incoming requests and outgoing responses.
macro_rules! layer {
	() => {
		::tower_http::trace::TraceLayer::new_for_http()
			.make_span_with($crate::middleware::logging::make_span_with)
			.on_request($crate::middleware::logging::on_request)
			.on_body_chunk($crate::middleware::logging::on_body_chunk)
			.on_eos(())
			.on_response($crate::middleware::logging::on_response)
	};
}

pub(crate) use layer;

pub fn make_span_with(request: &Request) -> tracing::Span {
	tracing::span! {
		Level::TRACE,
		"request",
		method = %request.method(),
		path = format_args!("`{}`", request.uri()),
		version = ?request.version(),
		request_headers = ?request.headers(),
		request_body = tracing::field::Empty,
		response_code = tracing::field::Empty,
		response_headers = tracing::field::Empty,
		response_body = tracing::field::Empty,
		latency = tracing::field::Empty,
	}
}

pub fn on_request(_request: &Request, _span: &tracing::Span) {
	// Currently a NOOP.
}

pub fn on_body_chunk(_chunk: &Bytes, _latency: Duration, _span: &tracing::Span) {
	// TODO(AlphaKeks): figure out how to identify chunks to log them correctly

	/// Turns a byte-slice into a string-slice with fallback values if the given `bytes` are
	/// either empty, or invalid UTF-8.
	///
	/// This is intended to be used with request/response payloads.
	const fn _stringify_bytes(bytes: &[u8]) -> &str {
		match std::str::from_utf8(bytes) {
			Ok(s) if s.is_empty() => "null",
			Ok(s) => s,
			Err(_) => "<bytes>",
		}
	}
}

pub fn on_response(response: &Response, latency: Duration, span: &tracing::Span) {
	span.record("response_code", format_args!("{}", response.status().as_u16()))
		.record("response_headers", format_args!("{:?}", response.headers()))
		.record("latency", format_args!("{latency:?}"));
}
