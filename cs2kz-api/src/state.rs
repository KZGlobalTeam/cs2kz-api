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
	/// This can be used to make database queries. See [`sqlx`].
	mysql_pool: MySqlPool,

	jwt: JwtState,
}

pub struct JwtState {
	/// Encodes [`GameServerInfo`](crate::middleware::auth::jwt::GameServerInfo) as a JWT.
	pub encode: jwt::EncodingKey,

	/// Header value for encoding JWTs.
	pub header: jwt::Header,

	/// Decodes a JWT into a [`GameServerInfo`](crate::middleware::auth::jwt::GameServerInfo).
	pub decode: jwt::DecodingKey,

	/// Validation struct for the JWT algorithm.
	pub validation: jwt::Validation,
}

impl AppState {
	/// Constructs a new [`AppState`].
	pub async fn new(database_url: &str, jwt_secret: &str) -> color_eyre::Result<Self> {
		let mysql_pool = MySqlPoolOptions::new()
			.connect(database_url)
			.await
			.context("Failed to establish database connection.")?;

		let jwt = JwtState {
			encode: jwt::EncodingKey::from_base64_secret(jwt_secret)
				.context("Failed to consturct JWT encoding key.")?,
			header: jwt::Header::default(),
			decode: jwt::DecodingKey::from_base64_secret(jwt_secret)
				.context("Failed to consturct JWT decoding key.")?,
			validation: jwt::Validation::default(),
		};

		Ok(Self { mysql_pool, jwt })
	}

	/// Returns a reference to the application's database connection pool.
	pub const fn database(&self) -> &MySqlPool {
		&self.mysql_pool
	}

	/// Returns a reference to the application's JWT data.
	pub const fn jwt(&self) -> &JwtState {
		&self.jwt
	}

	/// Starts a new MySQL transaction.
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
