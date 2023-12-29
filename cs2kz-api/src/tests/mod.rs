use std::fmt::Display;
use std::net::SocketAddr;

use ctor::ctor;
use sqlx::migrate::Migrator;
use sqlx::MySqlPool;
use tracing_subscriber::EnvFilter;
use url::Url;

mod status;
mod players;
mod maps;
mod servers;
mod gameserver_auth;

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
	pool: MySqlPool,
}

impl Context {
	fn new(addr: SocketAddr, pool: MySqlPool) -> Self {
		Self { client: reqwest::Client::new(), addr, pool }
	}

	/// Utility method for constructing a request URL to this test's API instance.
	fn url(&self, path: impl Display) -> Url {
		Url::parse(&format!("http://{}{}", self.addr, path)).unwrap()
	}
}
