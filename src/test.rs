use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr};
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
use crate::{Config, API};

/// Wrapper over std's `assert!()` macro that uses [`anyhow::ensure!()`] instead.
macro_rules! assert {
	($($t:tt)*) => {
		::anyhow::ensure!($($t)*)
	};
}

pub(crate) use assert;

/// Wrapper over std's `assert_eq!()` macro that uses [`anyhow::ensure!()`] instead.
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

/// Wrapper over std's `assert_ne!()` macro that uses [`anyhow::ensure!()`] instead.
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
	pub database: Pool<MySql>,

	/// A shutdown signal to have the API shutdown cleanly.
	shutdown: oneshot::Sender<()>,

	/// Handle to the API's background task.
	api_task: task::JoinHandle<anyhow::Result<()>>,

	/// Database container handle.
	database_container: ContainerAsync<Mariadb>,

	/// Header data to use when signing JWTs.
	jwt_header: jwt::Header,

	/// Secret key to use when signing JWTs.
	jwt_encoding_key: jwt::EncodingKey,

	/// Secret key to use when validating JWTs.
	jwt_decoding_key: jwt::DecodingKey,

	/// Extra validation steps when validating JWTs.
	jwt_validation: jwt::Validation,
}

impl Context {
	/// Create a new test context.
	///
	/// This is called in macro code and so should not be invoked manually.
	#[doc(hidden)]
	pub async fn new() -> anyhow::Result<Self> {
		let test_id = Uuid::now_v7();

		eprintln!("[{test_id}] setting up");
		eprintln!("[{test_id}] loading config");

		let config = Config::new()
			.map(Box::new)
			.map(Box::leak)
			.context("load config")?;

		let port = thread_rng().gen_range(5000..=50000);

		config.addr.set_port(port);
		config.public_url.set_port(Some(port)).unwrap();

		eprintln!("[{test_id}] starting database container");

		let database_container = Mariadb::default().start().await;
		let database_ip = match database_container.get_host().await {
			Host::Domain(domain) if domain == "localhost" => IpAddr::V4(Ipv4Addr::LOCALHOST),
			Host::Domain(domain) => anyhow::bail!("cannot use domain for database url ({domain})"),
			Host::Ipv4(ip) => IpAddr::V4(ip),
			Host::Ipv6(ip) => IpAddr::V6(ip),
		};
		let database_port = database_container.get_host_port_ipv4(3306).await;

		config.database_url.set_username("root").unwrap();
		config.database_url.set_password(None).unwrap();
		config.database_url.set_ip_host(database_ip).unwrap();
		config.database_url.set_port(Some(database_port)).unwrap();

		let config: &'static Config = config;
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
		let api_task = task::spawn(async move {
			eprintln!("[{test_id}] spawning API task");
			API::run_until(config.clone(), async move {
				_ = shutdown_rx.await;
			})
			.await
			.context("run api")?;

			anyhow::Ok(())
		});

		// let the API start up
		task::yield_now().await;

		eprintln!("[{test_id}] running test");

		Ok(Context {
			test_id,
			config,
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

	/// Run cleanup logic after a test.
	///
	/// This is called in macro code and so should not be invoked manually.
	#[doc(hidden)]
	pub async fn cleanup(self) -> anyhow::Result<()> {
		let Self {
			test_id,
			config,
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

		database_container.rm().await;

		eprintln!("[{test_id}] done");

		Ok(())
	}

	pub fn url<P>(&self, path: P) -> Url
	where
		P: Display,
	{
		self.config
			.public_url
			.join(&format!("{path}"))
			.expect("invalid url path")
	}

	pub fn auth_server(&self, expires_after: Duration) -> Result<String, jwt::errors::Error> {
		let server = authentication::Server::new(ServerID(1), PluginVersionID(1));

		self.encode_jwt(&server, expires_after)
	}

	pub async fn auth_session(&self, steam_id: SteamID) -> crate::Result<authentication::Session> {
		authentication::Session::create(steam_id, self.config, self.database.begin().await?).await
	}

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

	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>, jwt::errors::Error>
	where
		T: DeserializeOwned,
	{
		jwt::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation).map(|jwt| jwt.claims)
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

	for path in [".env.example", ".env", ".env.docker.example", ".env.docker"] {
		if let Err(err) = dotenvy::from_filename(path) {
			eprintln!("WARNING: Failed to load `{path}`: {err}");
		}
	}
}

#[crate::integration_test]
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
