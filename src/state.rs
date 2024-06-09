//! The API's main application state.
//!
//! This is initialized once on startup, and then passed around the application by axum.

use std::convert::Infallible;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use derive_more::Debug;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::pool::PoolOptions;
use sqlx::{MySql, Pool, Transaction};

use crate::authentication::Jwt;
use crate::{Error, Result};

/// The main application state.
///
/// A `'static` reference to this is passed around the application.
#[derive(Debug)]
pub struct State {
	/// The API configuration.
	pub config: crate::Config,

	/// Connection pool to the backing database.
	#[debug(skip)]
	pub database: Pool<MySql>,

	/// HTTP client for making requests to external APIs.
	#[debug(skip)]
	pub http_client: reqwest::Client,

	/// JWT related state.
	#[debug(skip)]
	jwt_state: JwtState,
}

impl State {
	/// The minimum amount of open database connections to keep in the connection pool.
	const MIN_DB_CONNECTIONS: u32 = if cfg!(production) { 200 } else { 20 };

	/// The maximum amount of open database connections to keep in the connection pool.
	const MAX_DB_CONNECTIONS: u32 = if cfg!(production) { 256 } else { 50 };

	/// Creates a new [`State`] object.
	pub async fn new(config: crate::Config) -> Result<Self> {
		let database = PoolOptions::new()
			.min_connections(Self::MIN_DB_CONNECTIONS)
			.max_connections(Self::MAX_DB_CONNECTIONS)
			.connect(config.database_url.as_str())
			.await?;

		let http_client = reqwest::Client::new();
		let jwt_state = JwtState::new(&config)?;

		Ok(Self {
			config,
			database,
			http_client,
			jwt_state,
		})
	}

	/// Begins a new database transaction.
	pub async fn transaction(&self) -> Result<Transaction<'_, MySql>> {
		self.database.begin().await.map_err(Error::from)
	}

	/// Encodes the given `payload` in a JWT that will expire after a given amount of time.
	pub fn encode_jwt<T>(&self, jwt: Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		self.jwt_state.encode(jwt)
	}

	/// Decodes the given `jwt` into some type `T`.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		self.jwt_state.decode(jwt)
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for &'static State {
	type Rejection = Infallible;

	async fn from_request_parts(
		_parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self, Self::Rejection> {
		Ok(state)
	}
}

/// JWT related state such as the secret key and algorithm information.
#[allow(missing_debug_implementations)]
struct JwtState {
	/// Header data to use when signing JWTs.
	jwt_header: jwt::Header,

	/// Secret key to use when signing JWTs.
	jwt_encoding_key: jwt::EncodingKey,

	/// Secret key to use when validating JWTs.
	jwt_decoding_key: jwt::DecodingKey,

	/// Extra validation steps when validating JWTs.
	jwt_validation: jwt::Validation,
}

impl JwtState {
	/// Creates a new [`JwtState`].
	fn new(config: &crate::Config) -> Result<Self> {
		let jwt_header = jwt::Header::default();

		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&config.jwt_secret)
			.map_err(|err| Error::encode_jwt(err))?;

		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&config.jwt_secret)
			.map_err(|err| Error::encode_jwt(err))?;

		let jwt_validation = jwt::Validation::default();

		Ok(Self {
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
		})
	}

	/// Encodes the given `payload` in a JWT that will expire after a given amount of time.
	fn encode<T>(&self, jwt: Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jwt::encode(&self.jwt_header, &jwt, &self.jwt_encoding_key)
			.map_err(|err| Error::encode_jwt(err))
	}

	/// Decodes the given `jwt` into some type `T`.
	fn decode<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		jwt::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation)
			.map(|jwt| jwt.claims)
			.map_err(|err| Error::invalid("jwt").context(err))
	}
}
