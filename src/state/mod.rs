use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, MySqlPool, Transaction};

use crate::auth::{steam, Jwt};
use crate::config::Environment;

mod error;
pub use error::{Error, Result};

mod jwt;

#[derive(Debug)]
pub struct State {
	config: crate::Config,
	connection_pool: MySqlPool,
	http_client: reqwest::Client,
	steam_login_form: steam::LoginForm,
	jwt: jwt::State,
}

impl State {
	pub async fn new(config: crate::Config) -> Result<Self> {
		let connection_pool = MySqlPoolOptions::new()
			.connect(config.database.url.as_str())
			.await?;

		let http_client = reqwest::Client::new();
		let steam_login_form = steam::LoginForm::new(config.public_url.clone());
		let jwt = jwt::State::new(&config.jwt.secret)?;

		Ok(Self { config, connection_pool, http_client, steam_login_form, jwt })
	}

	pub fn config(&self) -> &crate::Config {
		&self.config
	}

	pub fn in_dev(&self) -> bool {
		matches!(self.config.environment, Environment::Local)
	}

	pub fn in_prod(&self) -> bool {
		matches!(self.config.environment, Environment::Production)
	}

	pub fn domain(&self) -> String {
		self.config
			.public_url
			.domain()
			.map(|domain| format!(".{domain}"))
			.unwrap_or_else(|| {
				self.config
					.public_url
					.host_str()
					.map(ToOwned::to_owned)
					.expect("API url should have a host")
			})
	}

	pub fn database(&self) -> &MySqlPool {
		&self.connection_pool
	}

	pub fn http(&self) -> &reqwest::Client {
		&self.http_client
	}

	pub fn steam_login(&self) -> &steam::LoginForm {
		&self.steam_login_form
	}

	/// Begins a new SQL transaction.
	///
	/// If the transaction object returned by this function is dropped without calling
	/// [`Transaction::commit()`], it will be rolled back automatically.
	pub async fn transaction(&self) -> Result<Transaction<'static, MySql>> {
		self.connection_pool.begin().await.map_err(Error::from)
	}

	pub fn encode_jwt<T>(&self, payload: &Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jsonwebtoken::encode(&self.jwt.header, payload, &self.jwt.encoding_key).map_err(Into::into)
	}

	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<T>
	where
		T: DeserializeOwned,
	{
		jsonwebtoken::decode(jwt, &self.jwt.decoding_key, &self.jwt.validation)
			.map(|token| token.claims)
			.map_err(Into::into)
	}
}
