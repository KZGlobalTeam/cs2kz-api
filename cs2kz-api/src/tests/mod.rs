use std::fmt::Display;
use std::net::SocketAddr;

use color_eyre::eyre::eyre;
use color_eyre::Result;
use ctor::ctor;
use sqlx::migrate::Migrator;
use sqlx::MySqlPool;
use tokio::net::TcpListener;
use tokio::task;
use tracing_subscriber::EnvFilter;

use crate::{Config, API};

mod status;
mod players;
mod maps;
mod servers;

static MIGRATOR: Migrator = sqlx::migrate!("../database/migrations");

#[ctor]
fn setup() {
	color_eyre::install().expect("Failed to setup color-eyre.");
	dotenvy::dotenv().expect("Failed to find `.env` file.");
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env())
		.init();
}

/// Test Context.
///
/// This struct is used for running isolated instances of the API.
/// It holds a [`client`] field that you can use to make requests to the [API's URL][addr].
///
/// Use [`Context::url()`][url] for easily constructing URLs.
///
/// [`client`]: Context::client
/// [addr]: Context::addr
/// [url]: Context::url
struct Context {
	client: reqwest::Client,
	addr: SocketAddr,
}

impl Context {
	/// Creates a new test context.
	async fn new(database: MySqlPool) -> Result<Self> {
		let tcp_listener = TcpListener::bind("127.0.0.1:0").await?;
		let addr = tcp_listener.local_addr()?;
		let ctx = Self { client: reqwest::Client::new(), addr };
		let mut config = Config::new().await?;

		config.socket_addr.set_port(addr.port());
		config
			.api_url
			.set_port(Some(addr.port()))
			.map_err(|_| eyre!("Failed to set API port."))?;

		// Run the API in the background.
		task::spawn(async move {
			API::run(config, database, tcp_listener)
				.await
				.expect("Failed to run API.");

			unreachable!("API shutdown prematurely.");
		});

		Ok(ctx)
	}

	/// Utility method for constructing a request URL to this test's API instance.
	fn url(&self, path: impl Display) -> String {
		format!("http://{}{}", self.addr, path)
	}
}
