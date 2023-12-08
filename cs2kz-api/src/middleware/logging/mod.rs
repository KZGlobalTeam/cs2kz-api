use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::Response;
use tokio::time::Instant;
use tracing::Level;

use crate::{Error, Result};

/// Logs basic information about an incoming request.
pub async fn log_request(
	request_addr: ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<Response> {
	// Extract request body
	let (parts, body) = request.into_parts();
	let bytes = axum::body::to_bytes(body, usize::MAX)
		.await
		.map_err(|_| Error::InvalidRequestBody)?;

	// Log everything about the request and reserve fields for information about the response
	// we'll get later
	let span = tracing::info_span! {
		"log_request",
		"method" = %parts.method,
		"path" = %parts.uri,
		"request_addr" = %request_addr.0,
		"request_body" = stringify_bytes(&bytes),
		"response_status" = tracing::field::Empty,
		"response_body" = tracing::field::Empty,
	};

	// Re-construct the request and run the next service
	let body = Body::from(bytes);
	let request = Request::from_parts(parts, body);

	tracing::event!(Level::DEBUG, "starting to process request");

	let start = Instant::now();
	let response = next.run(request).await;

	tracing::event!(Level::DEBUG, took = ?start.elapsed(), "done processing request");

	// Split up the response and log the fields we reserved earlier
	let (parts, body) = response.into_parts();

	span.record("response_status", parts.status.to_string());

	let bytes = axum::body::to_bytes(body, usize::MAX)
		.await
		.expect("invalid response body");

	span.record("response_body", stringify_bytes(&bytes));

	// Re-construct the response and return
	let body = Body::from(bytes);
	let response = Response::from_parts(parts, body);

	Ok(response)
}

fn stringify_bytes(bytes: &[u8]) -> &str {
	match std::str::from_utf8(bytes) {
		Ok(s) if s.is_empty() => "null",
		Ok(s) => s,
		Err(_) => "<bytes>",
	}
}
