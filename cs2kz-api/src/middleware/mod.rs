use {
	crate::{Error, Result},
	axum::{body::Body, extract::Request},
	http_body_util::BodyExt,
	serde::de::DeserializeOwned,
};

pub mod auth;

/// Extracts some `T` as JSON from a request body.
pub async fn deserialize_body<T>(request: Request) -> Result<(T, Request)>
where
	T: DeserializeOwned, {
	let (parts, body) = request.into_parts();

	let bytes = body
		.collect()
		.await
		.map_err(|_| Error::InvalidRequestBody)?
		.to_bytes();

	let json = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRequestBody)?;
	let body = Body::from(bytes);
	let request = Request::from_parts(parts, body);

	Ok((json, request))
}
