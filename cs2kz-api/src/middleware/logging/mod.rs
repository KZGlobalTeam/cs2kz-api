use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::Response;
use http_body_util::BodyExt;
use serde_json::Value as JsonValue;

use crate::{Error, Result};

/// Logs basic information about an incoming request.
#[tracing::instrument(skip_all, fields(
	method = %request.method(),
	path = %request.uri(),
	request_addr = %addr,
	request_body,
	response_status,
	response_body,
))]
pub async fn log_request(
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<Response> {
	let span = tracing::Span::current();

	let (parts, body) = request.into_parts();

	let body = inspect_body(body, |json| {
		span.record("request_body", json.to_string());
	})
	.await?;

	let request = Request::from_parts(parts, body);
	let response = next.run(request).await;
	let (parts, body) = response.into_parts();

	span.record("response_status", parts.status.to_string());

	let body = inspect_body(body, |json| {
		span.record("response_body", json.to_string());
	})
	.await?;

	let response = Response::from_parts(parts, body);

	Ok(response)
}

/// Consumes an HTTP body and attempts to deserialize it as JSON.
/// If that succeeds, it calls the supplied `then` function with the JSON data.
async fn inspect_body<F>(body: Body, mut then: F) -> Result<Body>
where
	F: FnMut(JsonValue),
{
	let bytes = body
		.collect()
		.await
		.map_err(|_| Error::InvalidRequestBody)?
		.to_bytes();

	if let Ok(data) = serde_json::from_slice(&bytes) {
		then(data);
	}

	Ok(Body::from(bytes))
}
