use axum::body::Body;
use axum::extract::Request;
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;

use crate::{Error, Result};

pub mod auth;
pub mod logging;

/// Parses the request body as JSON into some type `T`.
///
/// Returns `Ok((None, Request))` if the body was empty.
pub async fn deserialize_body<T>(request: Request) -> Result<(Option<T>, Request)>
where
	T: DeserializeOwned,
{
	let (parts, body) = request.into_parts();

	let bytes = body
		.collect()
		.await
		.map_err(|_| Error::InvalidRequestBody)?
		.to_bytes();

	let json = if bytes.is_empty() {
		None
	} else {
		serde_json::from_slice::<T>(&bytes)
			.map(Some)
			.map_err(|_| Error::InvalidRequestBody)?
	};

	let body = Body::from(bytes);
	let request = Request::from_parts(parts, body);

	Ok((json, request))
}
