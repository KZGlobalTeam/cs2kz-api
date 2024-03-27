//! Everything for dealing with [JWTs].
//!
//! The main attraction in this module is the [`Jwt<T>`] struct. It is responsible for encoding and
//! decoding JWTs, and can act as an [extractor] in handlers.
//!
//! [JWTs]: https://jwt.io/introduction/
//! [extractor]: axum::extract

use std::fmt::{self, Debug};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::trace;
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

use crate::{Error, Result, State};

/// Helper struct for encoding / decoding JWTs.
#[derive(Serialize, Deserialize)]
pub struct Jwt<T> {
	/// The payload.
	#[serde(flatten)]
	payload: T,

	/// The expiration date.
	exp: u64,
}

impl<T> Jwt<T> {
	/// Creates a new [`Jwt<T>`] which will expire after a certain amount of time.
	pub fn new(payload: T, expires_after: Duration) -> Self {
		Self {
			payload,
			exp: jsonwebtoken::get_current_timestamp() + expires_after.as_secs(),
		}
	}

	/// Checks whether this JWT has already expired.
	pub fn has_expired(&self) -> bool {
		self.exp < jsonwebtoken::get_current_timestamp()
	}

	/// Returns the wrapped payload.
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

impl<T> Debug for Jwt<T>
where
	T: Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		Debug::fmt(&self.payload, f)
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
		let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
		let jwt = state.decode_jwt::<T>(header.token())?;

		if jwt.has_expired() {
			return Err(Error::expired_access_key());
		}

		trace!(target: "audit_log", token = %header.token(), "authenticated jwt");

		Ok(jwt)
	}
}

impl<'s, T> ToSchema<'s> for Jwt<T> {
	fn schema() -> (&'s str, RefOr<Schema>) {
		(
			"JWT",
			ObjectBuilder::new()
				.description(Some("https://jwt.io"))
				.schema_type(SchemaType::String)
				.build()
				.into(),
		)
	}
}
