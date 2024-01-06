//! The API's main application state.
//!
//! This is static data that will be passed to most handlers and middleware functions.

use std::fmt;

use axum::response::Redirect;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{MySql, MySqlPool, Transaction};
use url::Url;

use crate::config::Environment;
use crate::steam;

mod error;
pub use error::{Error, Result};

static STEAM_OPEN_ID_URL: &str = "https://steamcommunity.com/openid/login";

/// The API's main application state.
pub struct AppState {
	environment: Environment,
	database: MySqlPool,
	http_client: reqwest::Client,
	jwt_header: jwt::Header,
	jwt_encoding_key: jwt::EncodingKey,
	jwt_decoding_key: jwt::DecodingKey,
	jwt_validation: jwt::Validation,
	public_url: Url,
	steam_redirect_form: steam::RedirectForm,
}

impl AppState {
	/// Constructs an [`AppState`] object.
	#[tracing::instrument]
	pub async fn new(
		environment: Environment,
		database: MySqlPool,
		jwt_secret: String,
		api_url: Url,
	) -> Result<&'static Self> {
		let http_client = reqwest::Client::new();
		let jwt_header = jwt::Header::default();
		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&jwt_secret)?;
		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&jwt_secret)?;
		let jwt_validation = jwt::Validation::default();
		let steam_redirect_form = steam::RedirectForm::new(api_url.clone());

		let state = Self {
			environment,
			database,
			http_client,
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
			public_url: api_url,
			steam_redirect_form,
		};

		Ok(Box::leak(Box::new(state)))
	}

	/// Determines the environment the API is currently running in.
	pub const fn environment(&self) -> Environment {
		self.environment
	}

	/// Determines whether the API is currently running in development mode.
	pub const fn in_dev(&self) -> bool {
		matches!(self.environment, Environment::Development)
	}

	/// Determines whether the API is currently running in production.
	pub const fn in_prod(&self) -> bool {
		matches!(self.environment, Environment::Production)
	}

	/// Provides access to the API's database connection pool.
	pub const fn database(&self) -> &MySqlPool {
		&self.database
	}

	/// Provides access to an HTTP client for making requests to other APIs.
	pub const fn http_client(&self) -> &reqwest::Client {
		&self.http_client
	}

	/// Provides access to the API's public URL.
	pub const fn public_url(&self) -> &Url {
		&self.public_url
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
	pub fn steam_login(&self, origin_url: &Url) -> Redirect {
		let mut form = self.steam_redirect_form.clone();
		form.callback_url
			.query_pairs_mut()
			.append_pair("origin_url", origin_url.as_str());

		let steam_redirect_query = serde_urlencoded::to_string(form).expect("this is a valid form");
		let mut steam_redirect_url = Url::parse(STEAM_OPEN_ID_URL).expect("this is a valid url");
		steam_redirect_url.set_query(Some(&steam_redirect_query));

		Redirect::to(steam_redirect_url.as_str())
	}
}

impl fmt::Debug for AppState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("State").finish_non_exhaustive()
	}
}
