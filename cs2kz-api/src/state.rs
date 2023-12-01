use {
	crate::Result,
	color_eyre::eyre::Context,
	jsonwebtoken as jwt,
	sqlx::{mysql::MySqlPoolOptions, MySql, MySqlPool, Transaction},
	std::fmt::Debug,
};

/// Main application state.
///
/// This will be passed to every handler function that needs access to the API's database.
pub struct AppState {
	/// MySQL connection pool.
	///
	/// This can be used to make database queries.
	///
	/// See the [`sqlx`] crate for more details.
	database_pool: MySqlPool,

	/// JWT utilities for encoding / decoding.
	///
	/// See the [`jsonwebtoken`] crate for more details.
	jwt_state: JwtState,
}

impl AppState {
	/// Constructs a new [`AppState`].
	pub async fn new(database_url: &str, jwt_secret: &str) -> color_eyre::Result<Self> {
		let database_pool = MySqlPoolOptions::new()
			.connect(database_url)
			.await
			.context("Failed to establish database connection.")?;

		let jwt_state = JwtState::new(jwt_secret)?;

		Ok(Self { database_pool, jwt_state })
	}

	/// Returns a reference to the application's database connection pool.
	pub const fn database(&self) -> &MySqlPool {
		&self.database_pool
	}

	/// Returns a reference to the application's JWT data for encoding and decoding tokens.
	pub const fn jwt(&self) -> &JwtState {
		&self.jwt_state
	}

	/// Starts a new MySQL transaction.
	///
	/// Dropping the returned [`Transaction`] before calling [`.commit()`] will automatically
	/// roll it back.
	///
	/// See [`Transaction::drop`] for more information.
	///
	/// [`.commit()`]: sqlx::Transaction::commit
	pub async fn transaction(&self) -> Result<Transaction<'static, MySql>> {
		self.database().begin().await.map_err(Into::into)
	}
}

/// Required because we instrument all the handler functions.
/// We would have to explicitly `skip(state)` in all of them if [`AppState`] didn't implement
/// [`Debug`], but we also don't want to log the connection pool, so we just print "State".
impl Debug for AppState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("State")
	}
}

pub struct JwtState {
	/// Header value for encoding JWTs.
	pub header: jwt::Header,

	/// Encodes [`GameServerInfo`] as a JWT.
	///
	/// [`GameServerInfo`]: crate::middleware::auth::jwt::GameServerInfo
	pub encode: jwt::EncodingKey,

	/// Decodes a JWT into a [`GameServerInfo`].
	///
	/// [`GameServerInfo`]: crate::middleware::auth::jwt::GameServerInfo
	pub decode: jwt::DecodingKey,

	/// Validation struct for the JWT algorithm.
	pub validation: jwt::Validation,
}

impl JwtState {
	/// Constructs a new [`JwtState`] from the given `secret` key.
	fn new(secret: &str) -> color_eyre::Result<Self> {
		let header = jwt::Header::default();

		let encode = jwt::EncodingKey::from_base64_secret(secret)
			.context("Failed to consturct JWT encoding key.")?;

		let decode = jwt::DecodingKey::from_base64_secret(secret)
			.context("Failed to consturct JWT decoding key.")?;

		let validation = jwt::Validation::default();

		Ok(Self { header, encode, decode, validation })
	}
}
