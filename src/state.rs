//! The API's main application state.
//!
//! This is initialized once on startup, and then passed around the application by axum.

use std::time::Duration;

use derive_more::Debug;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{MySql, Pool};

use crate::auth::Jwt;
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

	/// Header data to use when signing JWTs.
	#[debug(skip)]
	jwt_header: jsonwebtoken::Header,

	/// Secret key to use when signing JWTs.
	#[debug(skip)]
	jwt_encoding_key: jsonwebtoken::EncodingKey,

	/// Secret key to use when validating JWTs.
	#[debug(skip)]
	jwt_decoding_key: jsonwebtoken::DecodingKey,

	/// Extra validation steps when validating JWTs.
	#[debug(skip)]
	jwt_validation: jsonwebtoken::Validation,
}

impl State {
	/// Creates a new [`State`] object and leaks it on the heap.
	///
	/// **This function should only ever be called once; it leaks memory.**
	pub async fn new(config: crate::Config) -> Result<&'static Self> {
		let database = Pool::connect(config.database_url.as_str()).await?;
		let http_client = reqwest::Client::new();
		let jwt_header = jsonwebtoken::Header::default();
		let jwt_encoding_key = jsonwebtoken::EncodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_decoding_key = jsonwebtoken::DecodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_validation = jsonwebtoken::Validation::default();

		Ok(Box::leak(Box::new(Self {
			config,
			database,
			http_client,
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
		})))
	}

	/// Encodes the given `payload` in a JWT that will expire after a given amount of time.
	pub fn encode_jwt(&self, payload: &impl Serialize, expires_after: Duration) -> Result<String> {
		jsonwebtoken::encode(
			&self.jwt_header,
			&Jwt::new(payload, expires_after),
			&self.jwt_encoding_key,
		)
		.map_err(|err| Error::from(err))
	}

	/// Decodes the given `jwt` into some type `T`.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		jsonwebtoken::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation)
			.map(|jwt| jwt.claims)
			.map_err(|err| Error::from(err))
	}
}
