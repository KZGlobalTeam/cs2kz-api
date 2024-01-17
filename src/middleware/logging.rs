use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tokio::time::Instant;
use tracing::trace;

use super::{Error, Result};
use crate::util;

/// Middleware for logging incoming requests and outgoing responses.
pub async fn layer(request: Request, next: Next) -> Result<Response> {
	// Break apart the request so we can consume the body.
	let (parts, body) = request.into_parts();
	let body = axum::body::to_bytes(body, usize::MAX)
		.await
		.map_err(Error::InvalidRequestBody)?;

	// Log relevant information.
	trace! {
		version = ?parts.version,
		method = %parts.method,
		path = %parts.uri,
		body = %util::stringify_bytes(&body),
		"processing request",
	};

	// Reconstruct the request so we can run the next service.
	let body = Body::from(body);
	let request = Request::from_parts(parts, body);

	// Run the next service and time how long it takes to complete.
	let start = Instant::now();
	let response = next.run(request).await;
	let took = start.elapsed();

	// Break apart the response so we can inspect the body.
	let (parts, body) = response.into_parts();
	let body = axum::body::to_bytes(body, usize::MAX)
		.await
		.expect("invalid response body");

	// Log relevant information.
	trace! {
		code = %parts.status,
		headers = ?parts.headers,
		body = %util::stringify_bytes(&body),
		took = ?took,
		"done processing request",
	};

	// Reconstruct the response.
	let body = Body::from(body);
	let response = Response::from_parts(parts, body);

	Ok(response)
}
