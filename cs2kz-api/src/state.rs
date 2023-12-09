use std::fmt::{self, Debug};

use axum::response::Redirect;
use color_eyre::eyre::Context;
use jsonwebtoken as jwt;
use jwt::TokenData;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, MySqlPool, Transaction};
use url::Url;

use crate::steam::RedirectForm;
use crate::Result;

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

	/// Static data used for communicating with the Steam API.
	steam_state: SteamState,

	/// HTTP client for making requests to other APIs.
	http_client: reqwest::Client,
}

impl AppState {
	/// Constructs a new [`AppState`].
	pub async fn new(
		database_url: &str,
		jwt_secret: &str,
		api_url: Url,
	) -> color_eyre::Result<Self> {
		let database_pool = MySqlPoolOptions::new()
			.connect(database_url)
			.await
			.context("Failed to establish database connection.")?;

		let jwt_state = JwtState::new(jwt_secret)?;
		let steam_state = SteamState::new(api_url)?;
		let http_client = reqwest::Client::new();

		Ok(Self { database_pool, jwt_state, steam_state, http_client })
	}

	/// Returns a reference to the application's database connection pool.
	pub const fn database(&self) -> &MySqlPool {
		&self.database_pool
	}

	/// Returns a reference to the application's JWT data for encoding and decoding tokens.
	pub const fn jwt(&self) -> &JwtState {
		&self.jwt_state
	}

	/// Returns a reference to the application's data about the Steam API.
	pub const fn steam(&self) -> &SteamState {
		&self.steam_state
	}

	pub const fn http_client(&self) -> &reqwest::Client {
		&self.http_client
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

/// Because [`AppState`] is used in nearly all handlers, and all handlers are instrumented, we
/// don't want to accidentally log the contents of [`AppState`]. Instead, we use a custom [`Debug`]
/// implementation that will simply not print anything.
///
/// Ideally every handler just includes `skip(state)` to not log it in the first place.
impl Debug for AppState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("State").finish_non_exhaustive()
	}
}

pub struct JwtState {
	/// Header value for encoding JWTs.
	header: jwt::Header,

	/// Encodes [`GameServerToken`] as a JWT.
	///
	/// [`GameServerToken`]: crate::middleware::auth::jwt::GameServerToken
	encode: jwt::EncodingKey,

	/// Decodes a JWT into a [`GameServerToken`].
	///
	/// [`GameServerToken`]: crate::middleware::auth::jwt::GameServerToken
	decode: jwt::DecodingKey,

	/// Validation struct for the JWT algorithm.
	validation: jwt::Validation,
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

	/// Encodes a payload using the server's JWT secret.
	pub fn encode<T>(&self, payload: &T) -> Result<String>
	where
		T: Serialize,
	{
		jwt::encode(&self.header, payload, &self.encode).map_err(Into::into)
	}

	/// Decodes a JWT using the server's secret.
	pub fn decode<T>(&self, token: &str) -> Result<TokenData<T>>
	where
		T: DeserializeOwned,
	{
		jwt::decode(token, &self.decode, &self.validation).map_err(Into::into)
	}
}

pub struct SteamState {
	redirect_url: Url,
}

impl SteamState {
	fn new(api_url: Url) -> color_eyre::Result<Self> {
		let redirect_form = RedirectForm::new(api_url, "/api/auth/steam_callback")?;
		let query_string = serde_urlencoded::to_string(redirect_form)?;
		let mut steam_url = Url::parse("https://steamcommunity.com/openid/login")?;
		steam_url.set_query(Some(&query_string));

		Ok(Self { redirect_url: steam_url })
	}

	/// Redirects a request to Steam's login page.
	pub fn redirect(&self) -> Redirect {
		Redirect::to(self.redirect_url.as_str())
	}
}
