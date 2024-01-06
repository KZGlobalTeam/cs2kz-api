//! This module holds middleware functions related to logging.

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::Response;
use tokio::time::Instant;
use tracing::trace;

use crate::{Error, Result};

/// Logs incoming requests and outgoing responses.
pub async fn log_request(
	request_addr: ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<Response> {
	let (parts, body) = request.into_parts();
	let bytes = axum::body::to_bytes(body, usize::MAX)
		.await
		.map_err(|_| Error::InvalidRequestBody)?;

	trace! {
		method = %parts.method,
		path = %parts.uri,
		request_addr = %request_addr.0,
		request_body = %stringify_bytes(&bytes),
		"processing request",
	};

	let body = Body::from(bytes);
	let request = Request::from_parts(parts, body);

	let start = Instant::now();
	let response = next.run(request).await;
	let took = start.elapsed();

	let (parts, body) = response.into_parts();
	let bytes = axum::body::to_bytes(body, usize::MAX)
		.await
		.expect("Invalid response body.");

	trace! {
		response_code = %parts.status,
		response_body = %stringify_bytes(&bytes),
		took = ?took,
		"done processing request",
	};

	let body = Body::from(bytes);
	let response = Response::from_parts(parts, body);

	Ok(response)
}

const fn stringify_bytes(bytes: &[u8]) -> &str {
	match std::str::from_utf8(bytes) {
		Ok(s) if s.is_empty() => "null",
		Ok(s) => s,
		Err(_) => "<bytes>",
	}
}
