//! JWT authentication.
//!
//! This module contains the [`Jwt`] type, which is used by [`State::encode_jwt()`] /
//! [`State::decode_jwt()`], and can be used as an [extractor].
//!
//! [extractor]: axum::extract

use std::panic::Location;
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
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

use crate::{Error, Result, State};

/// An extractor for JWTs.
#[derive(Debug, Deref, DerefMut, Serialize, Deserialize)]
pub struct Jwt<T> {
	/// The token payload.
	#[serde(flatten)]
	#[deref]
	#[deref_mut]
	#[debug("{payload:?}")]
	pub payload: T,

	/// The token's expiration date, as a unix timestamp.
	#[debug("{}", self.expires_on())]
	exp: u64,
}

impl<T> Jwt<T> {
	/// Creates a new JWT from the given `payload`, that will expire after the specified
	/// duration.
	#[track_caller]
	#[tracing::instrument(
		level = "debug",
		name = "authentication::jwt::new",
		skip(payload),
		fields(location = %Location::caller()),
	)]
	pub fn new(payload: T, expires_after: Duration) -> Self {
		Self {
			payload,
			exp: jwt::get_current_timestamp() + expires_after.as_secs(),
		}
	}

	/// Returns a timestamp of when this JWT will expire.
	pub fn expires_on(&self) -> DateTime<Utc> {
		let secs = i64::try_from(self.exp).expect("invalid expiration date");

		DateTime::from_timestamp(secs, 0).expect("invalid expiration date")
	}

	/// Checks if this JWT has expired.
	pub fn has_expired(&self) -> bool {
		self.exp < jwt::get_current_timestamp()
	}

	/// Turns this JWT into its inner payload.
	pub fn into_payload(self) -> T {
		self.payload
	}
}

#[async_trait]
impl<T> FromRequestParts<State> for Jwt<T>
where
	T: DeserializeOwned,
{
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::jwt::from_request_parts",
		skip_all,
		fields(token = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(parts: &mut request::Parts, state: &State) -> Result<Self> {
		let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
		let jwt = state
			.decode_jwt::<T>(header.token())
			.map_err(|err| Error::invalid("token").context(err))?;

		if jwt.has_expired() {
			return Err(Error::expired_key());
		}

		tracing::Span::current().record("token", header.token());
		tracing::debug!("authenticated JWT");

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
