//! Everything for dealing with [JWTs].
//!
//! The main attraction in this module is the [`Jwt<T>`] struct. It is responsible for encoding and
//! decoding JWTs, and can act as an [extractor] in handlers.
//!
//! [JWTs]: https://jwt.io/introduction/
//! [extractor]: axum::extract

use std::time::Duration;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use derive_more::{Debug, Deref, DerefMut};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::trace;
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

use crate::{Error, Result, State};

/// Helper struct for encoding / decoding JWTs.
#[derive(Debug, Deref, DerefMut, Serialize, Deserialize)]
pub struct Jwt<T> {
	/// The payload.
	#[serde(flatten)]
	#[deref]
	#[deref_mut]
	#[debug("{payload:?}")]
	pub payload: T,

	/// The expiration date.
	#[debug("{}", self.expires_on())]
	exp: u64,
}

impl<T> Jwt<T> {
	/// Creates a new [`Jwt<T>`] which will expire after a certain amount of time.
	pub fn new(payload: T, expires_after: Duration) -> Self {
		Self {
			payload,
			exp: jwt::get_current_timestamp() + expires_after.as_secs(),
		}
	}

	/// Returns the expiration date for this JWT.
	pub fn expires_on(&self) -> DateTime<Utc> {
		let secs = i64::try_from(self.exp).expect("invalid expiration date");

		DateTime::from_timestamp(secs, 0).expect("invalid expiration date")
	}

	/// Checks whether this JWT has already expired.
	pub fn has_expired(&self) -> bool {
		self.exp < jwt::get_current_timestamp()
	}

	/// Returns the wrapped payload.
	pub fn into_payload(self) -> T {
		self.payload
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
