//! The API's main application state.
//!
//! This is static data that will be passed to most handlers and middleware functions.

use std::fmt;

use axum::response::Redirect;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, MySqlPool, Transaction};
use url::Url;

use crate::steam;

mod error;
pub use error::{Error, Result};

static STEAM_OPEN_ID_URL: &str = "https://steamcommunity.com/openid/login";

/// The API's main application state.
pub struct AppState {
	database_pool: MySqlPool,
	http_client: reqwest::Client,
	jwt_header: jwt::Header,
	jwt_encoding_key: jwt::EncodingKey,
	jwt_decoding_key: jwt::DecodingKey,
	jwt_validation: jwt::Validation,
	steam_login_url: Url,
}

impl AppState {
	/// Constructs an [`AppState`] object.
	#[tracing::instrument]
	pub async fn new(database_url: Url, jwt_secret: String, api_url: Url) -> Result<&'static Self> {
		let database_pool = MySqlPoolOptions::new()
			.connect(database_url.as_str())
			.await?;

		let http_client = reqwest::Client::new();

		let jwt_header = jwt::Header::default();
		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&jwt_secret)?;
		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&jwt_secret)?;
		let jwt_validation = jwt::Validation::default();

		let steam_login_form = steam::RedirectForm::new(api_url);
		let steam_login_query = serde_urlencoded::to_string(steam_login_form)?;
		let mut steam_login_url = Url::parse(STEAM_OPEN_ID_URL)?;

		steam_login_url.set_query(Some(&steam_login_query));

		let state = Self {
			database_pool,
			http_client,
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
			steam_login_url,
		};

		Ok(Box::leak(Box::new(state)))
	}

	/// Provides access to the API's database connection pool.
	pub const fn database(&self) -> &MySqlPool {
		&self.database_pool
	}

	/// Provides access to an HTTP client for making requests to other APIs.
	pub const fn http_client(&self) -> &reqwest::Client {
		&self.http_client
	}

	/// Starts a new database transaction.
	///
	/// If the returned [`Transaction`] is dropped before calling [`.commit()`], it will
	/// be rolled back automatically.
	///
	/// [`.commit()`]: sqlx::Transaction::commit
	pub async fn begin_transaction(&self) -> crate::Result<Transaction<'static, MySql>> {
		self.database().begin().await.map_err(Into::into)
	}

	/// Encodes some payload as a JWT.
	pub fn encode_jwt<T: Serialize>(&self, payload: &T) -> crate::Result<String> {
		jwt::encode(&self.jwt_header, payload, &self.jwt_encoding_key).map_err(Into::into)
	}

	/// Decodes a JWT into a specified type.
	pub fn decode_jwt<T: DeserializeOwned>(&self, token: &str) -> crate::Result<T> {
		jwt::decode(token, &self.jwt_decoding_key, &self.jwt_validation)
			.map(|token| token.claims)
			.map_err(Into::into)
	}

	/// Generates a [`Redirect`] for logging a user into Steam.
	pub fn steam_login(&self) -> Redirect {
		Redirect::to(self.steam_login_url.as_str())
	}
}

impl fmt::Debug for AppState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("State").finish_non_exhaustive()
	}
}
