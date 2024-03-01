use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, MySqlPool, Transaction};

use crate::auth::{steam, Jwt};

mod error;
pub use error::{Error, Result};

pub mod jwt;

/// The API's global state.
#[derive(Debug)]
pub struct State {
	/// The API configuration.
	pub config: crate::Config,

	/// Database connection pool.
	pub database: MySqlPool,

	/// HTTP client for making requests to external services such as Steam.
	pub http_client: reqwest::Client,

	/// JWT secrets.
	pub jwt: jwt::State,

	/// A cached version of query parameters used for Steam authentication.
	steam_login_form: steam::LoginForm,
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

		Ok(Self { config, database: connection_pool, http_client, steam_login_form, jwt })
	}

	/// Begins a new SQL transaction.
	///
	/// If the transaction object returned by this function is dropped without calling
	/// [`Transaction::commit()`], it will be rolled back automatically.
	pub async fn begin_transaction(&self) -> Result<Transaction<'static, MySql>> {
		self.database.begin().await.map_err(Error::from)
	}

	/// Returns query parameters to redirect a user to Steam with.
	pub fn steam_login(&self) -> steam::LoginForm {
		self.steam_login_form.clone()
	}

	/// Encodes the given `payload` as a JWT using the API's secret.
	pub fn encode_jwt<T>(&self, payload: &Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		self.jwt.encode(payload).map_err(Error::JwtEncode)
	}

	/// Decodes the given `jwt` into the desired payload type `T`.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<T>
	where
		T: DeserializeOwned,
	{
		self.jwt.decode(jwt).map_err(Error::JwtDecode)
	}
}
