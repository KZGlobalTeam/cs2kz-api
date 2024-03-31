use std::fmt::Display;
use std::num::NonZeroU16;
use std::time::Duration;

use cs2kz::SteamID;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::MySqlPool;
use tokio::sync::oneshot;
use url::Url;
use uuid::Uuid;

use crate::auth::{self, Jwt};

/// Wrapper over std's `assert!()` macro that uses [`eyre::ensure!()`] instead.
macro_rules! assert {
	($($t:tt)*) => {
		::eyre::ensure!($($t)*)
	};
}

pub(crate) use assert;

/// Wrapper over std's `assert_eq!()` macro that uses [`eyre::ensure!()`] instead.
macro_rules! assert_eq {
	($left:expr, $right:expr $(,)?) => {
		if $left != $right {
			::eyre::bail!("assertion `left == right` failed\n  left: {:?}\n right: {:?}", $left, $right)
		}
	};
	($left:expr, $right:expr, $($t:tt)*) => {
		assert!($left == $right, $($t)*)
	};
}

pub(crate) use assert_eq;

/// Wrapper over std's `assert_ne!()` macro that uses [`eyre::ensure!()`] instead.
macro_rules! assert_ne {
	($left:expr, $right:expr $(,)?) => {
		if $left == $right {
			::eyre::bail!("assertion `left != right` failed\n  left: {:?}\n right: {:?}", $left, $right)
		}
	};
	($left:expr, $right:expr, $($t:tt)*) => {
		assert!($left != $right, $($t)*)
	};
}

pub(crate) use assert_ne;

/// Test "context" to take care of setup & cleanup for integration tests.
///
/// Every test will get its own database and API instance. This struct takes care of creating the
/// database and running migrations
pub(crate) struct Context {
	pub test_id: Uuid,

	/// API configuration.
	pub config: &'static crate::Config,

	/// An HTTP client for making requests.
	pub http_client: reqwest::Client,

	/// A database connection.
	pub database: MySqlPool,

	/// A shutdown signal to have the API shutdown cleanly.
	pub shutdown: oneshot::Sender<()>,

	/// Header data to use when signing JWTs.
	jwt_header: jsonwebtoken::Header,

	/// Secret key to use when signing JWTs.
	jwt_encoding_key: jsonwebtoken::EncodingKey,

	/// Secret key to use when validating JWTs.
	jwt_decoding_key: jsonwebtoken::DecodingKey,

	/// Extra validation steps when validating JWTs.
	jwt_validation: jsonwebtoken::Validation,
}

impl Context {
	pub fn new(
		test_id: Uuid,
		config: crate::Config,
		http_client: reqwest::Client,
		database: MySqlPool,
		shutdown: oneshot::Sender<()>,
	) -> eyre::Result<Self> {
		let config = Box::leak(Box::new(config));
		let jwt_header = jsonwebtoken::Header::default();
		let jwt_encoding_key = jsonwebtoken::EncodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_decoding_key = jsonwebtoken::DecodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_validation = jsonwebtoken::Validation::default();

		Ok(Context {
			test_id,
			config,
			http_client,
			database,
			shutdown,
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
		})
	}

	pub fn url(&self, path: impl Display) -> Url {
		self.config
			.public_url
			.join(&format!("{path}"))
			.expect("invalid url path")
	}

	pub fn auth_server(
		&self,
		expires_after: Duration,
	) -> Result<String, jsonwebtoken::errors::Error> {
		let server = auth::Server::new(NonZeroU16::new(1).unwrap(), NonZeroU16::new(1).unwrap());

		self.encode_jwt(&server, expires_after)
	}

	pub async fn auth_session(&self, steam_id: SteamID) -> crate::Result<auth::Session> {
		auth::Session::create(steam_id, &self.database, self.config).await
	}

	pub fn encode_jwt(
		&self,
		payload: &impl Serialize,
		expires_after: Duration,
	) -> Result<String, jsonwebtoken::errors::Error> {
		jsonwebtoken::encode(
			&self.jwt_header,
			&Jwt::new(payload, expires_after),
			&self.jwt_encoding_key,
		)
	}

	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>, jsonwebtoken::errors::Error>
	where
		T: DeserializeOwned,
	{
		jsonwebtoken::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation)
			.map(|jwt| jwt.claims)
	}
}

#[ctor::ctor]
fn setup() {
	use std::{env, io};

	use tracing_subscriber::fmt::format::FmtSpan;
	use tracing_subscriber::EnvFilter;

	if let Ok(rust_log) = env::var("RUST_TEST_LOG") {
		tracing_subscriber::fmt()
			.with_target(true)
			.with_writer(io::stderr)
			.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
			.compact()
			.with_env_filter(EnvFilter::new(rust_log))
			.init();
	}

	let _ = ::dotenvy::dotenv();
}

#[crate::test]
async fn hello_world(ctx: &Context) {
	let response = ctx
		.http_client
		.get(ctx.config.public_url.as_str())
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let schnose = response.text().await?;

	assert_eq!(schnose, "(͡ ͡° ͜ つ ͡͡°)");
}
