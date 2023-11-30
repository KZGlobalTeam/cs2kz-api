use {
	crate::{middleware, Result},
	axum::{
		extract::{ConnectInfo, Request},
		middleware::Next,
		response::Response,
	},
	serde_json::Value as JsonValue,
	std::{fmt::Write, net::SocketAddr},
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
	let mut message = format!("{method} `{uri}` from {addr}");

	let (body, request) = middleware::deserialize_body::<JsonValue>(request).await?;

	if let Some(value) = body {
		write!(&mut message, " with {value:#?}").expect("this never fails");
	}

	info!("{message}");

	Ok(next.run(request).await)
}
