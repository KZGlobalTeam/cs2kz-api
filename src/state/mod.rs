use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, MySqlPool, Transaction};

use crate::auth::{steam, Jwt};

mod error;
pub use error::{Error, Result};

mod jwt;

/// The API's global state.
#[derive(Debug)]
pub struct State {
	/// The API configuration.
	config: crate::Config,

	/// Database connection pool.
	connection_pool: MySqlPool,

	/// HTTP client for making requests to external services such as Steam.
	http_client: reqwest::Client,

	/// A cached version of query parameters used for Steam authentication.
	///
	/// See [`LoginForm::origin_url()`] for more details.
	///
	/// [`LoginForm::origin_url()`]: steam::LoginForm::origin_url
	steam_login_form: steam::LoginForm,

	/// JWT secrets.
	jwt: jwt::State,
}

impl State {
	/// Creates a new [State] instance with the given `config`.
	pub async fn new(config: crate::Config) -> Result<Self> {
		let connection_pool = MySqlPoolOptions::new()
			.connect(config.database.url.as_str())
			.await?;

		let http_client = reqwest::Client::new();
		let steam_login_form = steam::LoginForm::new(config.public_url.clone());
		let jwt = jwt::State::new(&config.jwt.secret)?;

		Ok(Self { config, connection_pool, http_client, steam_login_form, jwt })
	}

	pub const fn config(&self) -> &crate::Config {
		&self.config
	}

	pub fn domain(&self) -> &str {
		self.config.wildcard_domain.as_str()
	}

	pub const fn in_dev(&self) -> bool {
		self.config.in_dev()
	}

	pub const fn in_prod(&self) -> bool {
		self.config.in_prod()
	}

	pub const fn database(&self) -> &MySqlPool {
		&self.connection_pool
	}

	pub const fn http(&self) -> &reqwest::Client {
		&self.http_client
	}

	pub const fn steam_login(&self) -> &steam::LoginForm {
		&self.steam_login_form
	}

	/// Begins a new SQL transaction.
	///
	/// If the transaction object returned by this function is dropped without calling
	/// [`Transaction::commit()`], it will be rolled back automatically.
	pub async fn transaction(&self) -> Result<Transaction<'static, MySql>> {
		self.connection_pool.begin().await.map_err(Error::from)
	}

	/// Encodes the given `payload` as a JWT using the API's secret.
	pub fn encode_jwt<T>(&self, payload: &Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jsonwebtoken::encode(&self.jwt.header, payload, &self.jwt.encoding_key)
			.map_err(Error::JwtEncode)
	}

	/// Decodes the given `jwt` into the desired payload type `T`.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<T>
	where
		T: DeserializeOwned,
	{
		jsonwebtoken::decode(jwt, &self.jwt.decoding_key, &self.jwt.validation)
			.map(|token| token.claims)
			.map_err(Error::JwtDecode)
	}
}
