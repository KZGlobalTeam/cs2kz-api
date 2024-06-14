//! The API's global application state.
//!
//! A [`State`] instance is created on startup and then passed to axum so it can be accessed in
//! handlers, [middleware], [extractors], etc.
//!
//! [middleware]: axum::middleware
//! [extractors]: axum::extract

use std::convert::Infallible;
use std::sync::Arc;

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

/// The API's state.
#[derive(Debug, Clone)]
pub struct State {
	/// Runtime configuration.
	#[debug(skip)]
	pub config: Arc<crate::Config>,

	/// Database connection pool.
	#[debug(skip)]
	pub database: Pool<MySql>,

	/// An HTTP client for making requests to other APIs.
	#[debug(skip)]
	pub http_client: reqwest::Client,

	/// JWT state for encoding/decoding tokens.
	#[debug(skip)]
	jwt_state: Arc<JwtState>,
}

impl State {
	/// The minimum number of [database pool] connections.
	///
	/// [database pool]: State::database
	const MIN_DB_CONNECTIONS: u32 = if cfg!(production) { 200 } else { 20 };

	/// The maximum number of [database pool] connections.
	///
	/// [database pool]: State::database
	const MAX_DB_CONNECTIONS: u32 = if cfg!(production) { 256 } else { 50 };

	/// Creates a new [`State`].
	pub async fn new(api_config: crate::Config) -> Result<Self> {
		tracing::debug!(?api_config, "initializing application state");
		tracing::debug! {
			url = %api_config.database_url,
			min_connections = Self::MIN_DB_CONNECTIONS,
			max_connections = Self::MAX_DB_CONNECTIONS,
			"establishing database connection",
		};

		let config = Arc::new(api_config);
		let database = PoolOptions::new()
			.min_connections(Self::MIN_DB_CONNECTIONS)
			.max_connections(Self::MAX_DB_CONNECTIONS)
			.connect(config.database_url.as_str())
			.await?;

		let http_client = reqwest::Client::new();
		let jwt_state = JwtState::new(&config).map(Arc::new)?;

		Ok(Self {
			config,
			database,
			http_client,
			jwt_state,
		})
	}

	/// Begins a database transaction.
	///
	/// If the returned [`Transaction`] gets dropped without [`Transaction::commit()`] or
	/// [`Transaction::rollback()`] being called, it will be rolled back.
	pub async fn transaction(&self) -> Result<Transaction<'_, MySql>> {
		self.database.begin().await.map_err(Error::from)
	}

	/// Encodes a JWT.
	pub fn encode_jwt<T>(&self, jwt: Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		self.jwt_state.encode(jwt)
	}

	/// Decodes a JWT.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		self.jwt_state.decode(jwt)
	}
}

#[async_trait]
impl FromRequestParts<State> for State {
	type Rejection = Infallible;

	async fn from_request_parts(
		_parts: &mut request::Parts,
		state: &State,
	) -> Result<Self, Self::Rejection> {
		Ok(state.clone())
	}
}

/// JWT state for encoding/decoding tokens.
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
struct JwtState {
	jwt_header: jwt::Header,
	jwt_encoding_key: jwt::EncodingKey,
	jwt_decoding_key: jwt::DecodingKey,
	jwt_validation: jwt::Validation,
}

impl JwtState {
	/// Creates a new [`JwtState`].
	fn new(api_config: &crate::Config) -> Result<Self> {
		let jwt_header = jwt::Header::default();

		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&api_config.jwt_secret)
			.map_err(|err| Error::encode_jwt(err))?;

		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&api_config.jwt_secret)
			.map_err(|err| Error::encode_jwt(err))?;

		let jwt_validation = jwt::Validation::default();

		Ok(Self {
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
		})
	}

	/// Encodes a JWT.
	fn encode<T>(&self, jwt: Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jwt::encode(&self.jwt_header, &jwt, &self.jwt_encoding_key)
			.map_err(|err| Error::encode_jwt(err))
	}

	/// Decodes a JWT.
	fn decode<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		jwt::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation)
			.map(|jwt| jwt.claims)
			.map_err(|err| Error::invalid("jwt").context(err))
	}
}
