//! [JWT] authentication for CS2 servers.
//!
//! For a quick overview, see the [`auth` top-level documentation].
//!
//! [JWT]: https://jwt.io
//! [`auth` top-level documentation]: crate::services::auth

use std::time::Duration;
use std::{fmt, ops};

use axum::extract::{FromRef, FromRequestParts};
use axum::{async_trait, RequestPartsExt};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::services::AuthService;

mod rejection;
pub use rejection::JwtRejection;

mod service;
pub use service::{JwtLayer, JwtService};

mod server_info;
pub use server_info::ServerInfo;

/// A JWT.
///
/// This type can be used for encoding/decoding raw JWTs, and as an [extractor].
///
/// [extractor]: axum::extract
#[derive(Clone, Serialize, Deserialize)]
pub struct Jwt<T>
{
	/// The payload to encode in the token.
	#[serde(flatten)]
	payload: T,

	/// Timestamp (in seconds) of when this token will expire.
	#[serde(rename = "exp")]
	expiration_timestamp: u64,
}

impl<T> Jwt<T>
{
	/// Creates a new [`Jwt`].
	///
	/// You can encode it into a string using [`AuthService::encode_jwt()`].
	///
	/// [`AuthService::encode_jwt()`]: crate::services::AuthService::encode_jwt
	pub fn new(payload: T, expires_after: Duration) -> Self
	{
		Self {
			payload,
			expiration_timestamp: jsonwebtoken::get_current_timestamp() + expires_after.as_secs(),
		}
	}

	/// Returns a reference to the inner payload.
	pub fn payload(&self) -> &T
	{
		&self.payload
	}

	/// Returns a mutable reference to the inner payload.
	pub fn payload_mut(&mut self) -> &mut T
	{
		&mut self.payload
	}

	/// Returns the inner payload.
	pub fn into_payload(self) -> T
	{
		self.payload
	}

	/// Returns a unix timestamp of when this token will expire.
	pub fn timestamp(&self) -> u64
	{
		self.expiration_timestamp
	}

	/// Returns a [`chrono::DateTime`] of when this token will expire.
	pub fn expires_on(&self) -> DateTime<Utc>
	{
		let secs = i64::try_from(self.expiration_timestamp).expect("sensible expiration date");

		DateTime::from_timestamp(secs, 0).expect("valid expiration date")
	}

	/// Checks if this token has expired.
	pub fn has_expired(&self) -> bool
	{
		self.expiration_timestamp <= jsonwebtoken::get_current_timestamp()
	}
}

impl<T> fmt::Debug for Jwt<T>
where
	T: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Jwt")
			.field("payload", self.payload())
			.field("expires_on", &format_args!("{}", self.expires_on().format("%Y/%m/%d %H:%M:%S")))
			.finish()
	}
}

impl<T> ops::Deref for Jwt<T>
{
	type Target = T;

	fn deref(&self) -> &Self::Target
	{
		self.payload()
	}
}

impl<T> ops::DerefMut for Jwt<T>
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		self.payload_mut()
	}
}

#[async_trait]
impl<T, S> FromRequestParts<S> for Jwt<T>
where
	T: fmt::Debug + DeserializeOwned + Send + Sync + 'static,
	S: Send + Sync + 'static,
	AuthService: FromRef<S>,
{
	type Rejection = JwtRejection;

	#[tracing::instrument(
		name = "Jwt::from_request_parts",
		skip_all,
		fields(payload = tracing::field::Empty),
		err(Debug, level = "debug")
	)]
	async fn from_request_parts(
		parts: &mut http::request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		if let Some(jwt) = parts.extensions.remove::<Self>() {
			return Ok(jwt);
		}

		let auth_svc = AuthService::from_ref(state);

		let header = parts
			.extract::<TypedHeader<Authorization<Bearer>>>()
			.await?;

		let jwt = auth_svc.decode_jwt::<T>(header.token())?;

		tracing::Span::current().record("payload", format_args!("{:?}", jwt.payload()));

		if jwt.has_expired() {
			return Err(JwtRejection::JwtExpired);
		}

		Ok(jwt)
	}
}

#[cfg(test)]
mod tests
{
	use axum::RequestExt;
	use serde::{Deserialize, Serialize};
	use sqlx::{MySql, Pool};

	use super::*;
	use crate::services::auth::Error as AuthError;
	use crate::testing;

	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct TestInfo
	{
		foo: i32,
		bar: bool,
	}

	#[sqlx::test]
	async fn reject_missing_header(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);
		let mut req = http::Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Jwt<TestInfo>, _>(&auth_svc)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, JwtRejection::Header(ref rej) if rej.is_missing());

		Ok(())
	}

	#[sqlx::test]
	async fn reject_invalid_header(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);
		let mut req = http::Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", "foobarbaz")
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Jwt<TestInfo>, _>(&auth_svc)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, JwtRejection::Header(ref rej) if !rej.is_missing());

		Ok(())
	}

	#[sqlx::test]
	async fn reject_malformed_jwt(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);
		let mut req = http::Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", "Bearer not-a-jwt")
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Jwt<TestInfo>, _>(&auth_svc)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, JwtRejection::Auth(AuthError::DecodeJwt { .. }));

		Ok(())
	}

	#[sqlx::test]
	async fn reject_expired_jwt(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);
		let info = TestInfo { foo: 69, bar: true };
		let expires_after = Duration::from_secs(1);
		let jwt = auth_svc.encode_jwt(Jwt::new(info, expires_after))?;

		// jwt only has second precision, so we need to wait for it to expire
		tokio::time::sleep(Duration::from_secs(2)).await;

		let mut req = http::Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", format!("Bearer {jwt}"))
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Jwt<TestInfo>, _>(&auth_svc)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, JwtRejection::JwtExpired);

		Ok(())
	}

	#[sqlx::test]
	async fn accept_valid_jwt(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let auth_svc = testing::auth_svc(database);
		let info = TestInfo { foo: 69, bar: true };
		let expires_after = Duration::from_secs(69);
		let jwt = auth_svc.encode_jwt(Jwt::new(&info, expires_after))?;

		let mut req = http::Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Authorization", format!("Bearer {jwt}"))
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Jwt<TestInfo>, _>(&auth_svc)
			.await
			.unwrap();

		testing::assert_eq!(extracted.payload(), &info);

		Ok(())
	}
}
