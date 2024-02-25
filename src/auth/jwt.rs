use std::ops::{Deref, DerefMut};

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::Duration;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{audit, Error, Result, State};

/// Utility type for handling JWT payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwt<T> {
	/// The JWT's payload.
	#[serde(flatten)]
	payload: T,

	/// The expiration timestamp.
	exp: u64,
}

impl<T> Jwt<T> {
	/// Constructs a new JWT which will expire after a certain amount of time.
	pub fn new(payload: T, expires_after: Duration) -> Self {
		let expires_after: u64 = expires_after
			.num_seconds()
			.try_into()
			.expect("positive amount of seconds");

		let exp = jsonwebtoken::get_current_timestamp() + expires_after;

		Self { payload, exp }
	}

	/// Checks whether this JWT has already expired.
	pub fn has_expired(&self) -> bool {
		self.exp < jsonwebtoken::get_current_timestamp()
	}

	/// Consumes the JWT and returns back the payload.
	pub fn into_payload(self) -> T {
		self.payload
	}
}

impl<T> Deref for Jwt<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.payload
	}
}

impl<T> DerefMut for Jwt<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.payload
	}
}

#[async_trait]
impl<T> FromRequestParts<&'static State> for Jwt<T>
where
	T: DeserializeOwned,
{
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let (original, jwt) =
			TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
				.await
				.map_err(Error::from)
				.and_then(|jwt| {
					let decoded = state.decode_jwt::<Self>(jwt.token())?;

					Ok((jwt, decoded))
				})?;

		if jwt.has_expired() {
			return Err(Error::invalid("token")
				.with_detail("token is expired")
				.unauthorized());
		}

		audit!("jwt authenticated", token = %original.token());

		Ok(jwt)
	}
}
