//! Utilities for unit & integration tests.

use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use anyhow::Context as _;
use cs2kz::SteamID;
use rand::{thread_rng, Rng};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::migrate::MigrateDatabase;
use sqlx::{MySql, Pool};
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::mariadb::Mariadb;
use tokio::sync::oneshot;
use tokio::task;
use url::{Host, Url};
use uuid::Uuid;

use crate::authentication::{self, Jwt};
use crate::plugin::PluginVersionID;
use crate::servers::ServerID;
use crate::{steam, Config, Result};

/// Replacement for the builtin [`assert!()`] macro that uses [`anyhow::ensure!()`] instead.
macro_rules! assert {
	($($t:tt)*) => {
		::anyhow::ensure!($($t)*)
	};
}

pub(crate) use assert;

/// Replacement for the builtin [`assert_eq!()`] macro that uses [`anyhow::ensure!()`] instead.
macro_rules! assert_eq {
	($left:expr, $right:expr $(,)?) => {
		if $left != $right {
			::anyhow::bail!("assertion `left == right` failed\n  left: {:?}\n right: {:?}", $left, $right)
		}
	};
	($left:expr, $right:expr, $($t:tt)*) => {
		assert!($left == $right, $($t)*)
	};
}

pub(crate) use assert_eq;

/// Replacement for the builtin [`assert_ne!()`] macro that uses [`anyhow::ensure!()`] instead.
macro_rules! assert_ne {
	($left:expr, $right:expr $(,)?) => {
		if $left == $right {
			::anyhow::bail!("assertion `left != right` failed\n  left: {:?}\n right: {:?}", $left, $right)
		}
	};
	($left:expr, $right:expr, $($t:tt)*) => {
		assert!($left != $right, $($t)*)
	};
}

pub(crate) use assert_ne;

/// Test context.
///
/// An instance of this struct is passed to every integration test.
#[allow(clippy::missing_docs_in_private_items)]
pub(crate) struct Context {
	/// The test's ID.
	pub test_id: Uuid,

	/// The configuration to pass to this test's API instance.
	pub api_config: crate::Config,

	/// An HTTP client for making API requests.
	pub http_client: reqwest::Client,

	/// Database connection pool.
	pub database: Pool<MySql>,

	/// Shutdown signal to gracefully shutdown the API at the end of a test.
	shutdown: oneshot::Sender<()>,

	/// The [`tokio::task`] running the API.
	api_task: task::JoinHandle<anyhow::Result<()>>,

	/// Handle to a docker container running a database exclusively for this test.
	database_container: ContainerAsync<Mariadb>,

	jwt_header: jwt::Header,
	jwt_encoding_key: jwt::EncodingKey,
	jwt_decoding_key: jwt::DecodingKey,
	jwt_validation: jwt::Validation,
}

impl Context {
	/// Creates a new [`Context`].
	///
	/// This is used by macro code and should not be called manually.
	#[doc(hidden)]
	pub async fn new() -> anyhow::Result<Self> {
		let test_id = Uuid::now_v7();

		eprintln!("[{test_id}] setting up");
		eprintln!("[{test_id}] loading config");

		let mut config = Config::new().context("load config")?;
		let port = thread_rng().gen_range(5000..=50000);

		config.addr.set_port(port);
		config.public_url.set_port(Some(port)).unwrap();

		eprintln!("[{test_id}] starting database container");

		let database_container = Mariadb::default()
			.start()
			.await
			.with_context(|| format!("[{test_id}] start mariadb container"))?;

		let database_host = database_container
			.get_host()
			.await
			.with_context(|| format!("[{test_id}] get mariadb container host"))?;

		let database_ip = match database_host {
			Host::Domain(domain) if domain == "localhost" => IpAddr::V4(Ipv4Addr::LOCALHOST),
			Host::Domain(domain) => anyhow::bail!("cannot use domain for database url ({domain})"),
			Host::Ipv4(ip) => IpAddr::V4(ip),
			Host::Ipv6(ip) => IpAddr::V6(ip),
		};

		let database_port = database_container
			.get_host_port_ipv4(3306)
			.await
			.with_context(|| format!("[{test_id}] get mariadb container port"))?;

		config.database_url.set_username("root").unwrap();
		config.database_url.set_password(None).unwrap();
		config.database_url.set_ip_host(database_ip).unwrap();
		config.database_url.set_port(Some(database_port)).unwrap();

		let http_client = reqwest::Client::new();
		let jwt_header = jwt::Header::default();
		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_validation = jwt::Validation::default();

		eprintln!("[{test_id}] creating database");

		sqlx::MySql::create_database(config.database_url.as_str())
			.await
			.context("create database")?;

		eprintln!("[{test_id}] connecting to database");

		let database = sqlx::Pool::connect(config.database_url.as_str())
			.await
			.context("connect to database")?;

		eprintln!("[{test_id}] running migrations");

		sqlx::migrate!("./database/migrations")
			.run(&database)
			.await
			.context("run migrations")?;

		let (shutdown, shutdown_rx) = oneshot::channel();
		let api_task = task::spawn({
			eprintln!("[{test_id}] spawning API task");

			let fut = crate::run_until(config.clone(), async move {
				_ = shutdown_rx.await;
			});

			async move { fut.await.context("run api") }
		});

		// let the API start up
		task::yield_now().await;

		eprintln!("[{test_id}] running test");

		Ok(Context {
			test_id,
			api_config: config,
			http_client,
			database,
			shutdown,
			api_task,
			database_container,
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
		})
	}

	/// Performs cleanup after a successful test.
	///
	/// This is used by macro code and should not be called manually.
	#[doc(hidden)]
	pub async fn cleanup(self) -> anyhow::Result<()> {
		let Self {
			test_id,
			api_config: config,
			shutdown,
			api_task,
			database_container,
			..
		} = self;

		eprintln!("[{test_id}] sending shutdown signal");

		shutdown.send(()).expect("send shutdown signal");

		eprintln!("[{test_id}] dropping database");

		sqlx::MySql::drop_database(config.database_url.as_str())
			.await
			.context("drop database")?;

		api_task
			.await
			.context("api panicked")?
			.context("wait for api to shut down")?;

		eprintln!("[{test_id}] destroying database container");

		database_container
			.rm()
			.await
			.with_context(|| format!("[{test_id}] destroy mariadb container"))?;

		eprintln!("[{test_id}] done");

		Ok(())
	}

	/// Generates a URL for the given `path` that can be used to make an API request.
	pub fn url<P>(&self, path: P) -> Url
	where
		P: Display,
	{
		self.api_config
			.public_url
			.join(&format!("{path}"))
			.expect("invalid url path")
	}

	/// Generates a JWT for a fake CS2 server.
	pub fn auth_server(&self, expires_after: Duration) -> Result<String, jwt::errors::Error> {
		let server = authentication::Server::new(ServerID(1), PluginVersionID(1));

		self.encode_jwt(&server, expires_after)
	}

	/// Generates a fake session for the player with the given `steam_id`.
	pub async fn auth_session(&self, steam_id: SteamID) -> Result<authentication::Session> {
		let user = steam::User::invalid(steam_id);

		authentication::Session::create(
			&user,
			Ipv6Addr::LOCALHOST.into(),
			&self.api_config,
			self.database.begin().await?,
		)
		.await
	}

	/// Encodes a JWT.
	pub fn encode_jwt<T>(
		&self,
		payload: &T,
		expires_after: Duration,
	) -> Result<String, jwt::errors::Error>
	where
		T: Serialize,
	{
		jwt::encode(
			&self.jwt_header,
			&Jwt::new(payload, expires_after),
			&self.jwt_encoding_key,
		)
	}

	/// Decodes a JWT.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>, jwt::errors::Error>
	where
		T: DeserializeOwned,
	{
		jwt::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation).map(|jwt| jwt.claims)
	}
}

/// This function runs before every test to set up things like logging.
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

	[".env.example", ".env", ".env.docker.example", ".env.docker"]
		.into_iter()
		.filter_map(|path| dotenvy::from_filename(path).err().map(|err| (path, err)))
		.for_each(|(path, err)| eprintln!("WARNING: Failed to load `{path}`: {err}"));
}

#[crate::integration_test]
async fn hello_world(ctx: &Context) {
	let response = ctx
		.http_client
		.get(ctx.api_config.public_url.as_str())
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let schnose = response.text().await?;

	assert_eq!(schnose, "(͡ ͡° ͜ つ ͡͡°)");
}
