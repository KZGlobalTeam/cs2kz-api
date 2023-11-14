use {
	crate::{Error, Result},
	axum::{body::Body, http::Request},
	serde::de::DeserializeOwned,
};

pub mod auth;

/// Extracts some `T` as JSON from a request body.
pub async fn extract_from_body<T>(request: Request<Body>) -> Result<(T, Request<Body>)>
where
	T: DeserializeOwned, {
	let (parts, body) = request.into_parts();
	let bytes = hyper::body::to_bytes(body)
		.await
		.map_err(|_| Error::InvalidRequestBody)?;

	let json = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRequestBody)?;

	Ok((json, Request::from_parts(parts, bytes.into())))
}
