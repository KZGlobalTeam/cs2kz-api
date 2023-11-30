use {
	crate::{middleware, Result},
	axum::{
		extract::{ConnectInfo, Request},
		middleware::Next,
		response::Response,
	},
	serde_json::Value as JsonValue,
	std::net::SocketAddr,
	tracing::info,
};

/// Logs basic information about an incoming request.
pub async fn log_request(
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<Response> {
	let method = request.method();
	let uri = request.uri();

	info!("{method} `{uri}` from {addr}");

	Ok(next.run(request).await)
}

/// Logs basic information about an incoming request **including the request body**.
///
/// NOTE: This will **not** work if the request body cannot be deserialized into JSON.
#[tracing::instrument(skip(next), err)]
pub async fn log_request_with_body(
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<Response> {
	let (body, request) = middleware::deserialize_body::<JsonValue>(request).await?;
	let method = request.method();
	let uri = request.uri();

	info!("{method} `{uri}` from {addr} with body {body:#?}");

	Ok(next.run(request).await)
}
