use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{Error, Result, State};

/// Generic JWT wrapper.
///
/// This can be used for serializing / deserializing JWTs.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Serialize, Deserialize)]
pub struct JWT<Payload> {
	#[serde(flatten)]
	pub payload: Payload,

	exp: u64,
}

impl<Payload> JWT<Payload> {
	pub fn new(payload: Payload, expires_on: DateTime<Utc>) -> Self {
		Self { payload, exp: expires_on.timestamp() as _ }
	}

	/// Checks whether this token has expired.
	pub fn has_expired(&self) -> bool {
		self.exp < jsonwebtoken::get_current_timestamp()
	}

	/// Consumes the wrapper and returns the inner payload.
	pub fn into_inner(self) -> Payload {
		self.payload
	}
}

impl<Payload> Deref for JWT<Payload> {
	type Target = Payload;

	fn deref(&self) -> &Self::Target {
		&self.payload
	}
}

impl<Payload> DerefMut for JWT<Payload> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.payload
	}
}

#[async_trait]
impl<Payload> FromRequestParts<Arc<State>> for JWT<Payload>
where
	Payload: DeserializeOwned,
{
	type Rejection = Error;

	async fn from_request_parts(parts: &mut request::Parts, state: &Arc<State>) -> Result<Self> {
		let token = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
			.await
			.map_err(|_| Error::Unauthorized)?;

		let jwt = state.decode_jwt::<Self>(token.token())?;

		if jwt.has_expired() {
			return Err(Error::ExpiredToken);
		}

		Ok(jwt)
	}
}
